use std::io::Write;

use serde_json::{Map, Value};

use super::Session;
use crate::artifacts::{snapshot_entry, snapshot_value, validate_snapshot_token};
use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{
    require_keys, required_safe_relative_path, required_string, validate_safe_relative_path,
};
use crate::filesystem::descriptors::{file_identity, open_exclusive_at, rename_at, sync_directory};
use crate::filesystem::publication::assert_owned_temporary;
#[cfg(not(test))]
use crate::filesystem::publication::cleanup_owned_temp;
#[cfg(test)]
use crate::filesystem::publication::{
    apply_publish_test_fault, cleanup_owned_temp_with_test_fault, PublishTestFault,
};
use crate::protocol::{decode_base64url_bounded, json_object};
use crate::release_records::{
    expected_delivery_set_project_path, parse_delivery_set_record, parse_pointer_record,
    pointer_delivery_set_relative, validate_base_delivery_set_hash, validate_pointer_delivery_set,
};
use crate::startup::SessionPurpose;

const MAX_POINTER_BYTES: usize = 1024 * 1024;

impl Session {
    pub(super) fn publish_pointer(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose == SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "publishPointer is not available in a measurement session",
            ));
        }
        #[cfg(test)]
        {
            self.publish_pointer_inner(object, PublishTestFault::None)
        }
        #[cfg(not(test))]
        self.publish_pointer_inner(object)
    }

    #[cfg(test)]
    pub(crate) fn publish_pointer_with_test_fault(
        &mut self,
        object: &Map<String, Value>,
        fault: PublishTestFault,
    ) -> Result<Value> {
        self.publish_pointer_inner(object, fault)
    }

    #[cfg(test)]
    fn publish_pointer_inner(
        &mut self,
        object: &Map<String, Value>,
        fault: PublishTestFault,
    ) -> Result<Value> {
        self.publish_pointer_inner_impl(object, fault)
    }

    #[cfg(not(test))]
    fn publish_pointer_inner(&mut self, object: &Map<String, Value>) -> Result<Value> {
        self.publish_pointer_inner_impl(object)
    }

    #[cfg(test)]
    fn publish_pointer_inner_impl(
        &mut self,
        object: &Map<String, Value>,
        fault: PublishTestFault,
    ) -> Result<Value> {
        self.publish_pointer_body(object, Some(fault))
    }

    #[cfg(not(test))]
    fn publish_pointer_inner_impl(&mut self, object: &Map<String, Value>) -> Result<Value> {
        self.publish_pointer_body(object)
    }

    fn publish_pointer_body(
        &mut self,
        object: &Map<String, Value>,
        #[cfg(test)] fault: Option<PublishTestFault>,
    ) -> Result<Value> {
        require_keys(
            object,
            &[
                "id",
                "op",
                "path",
                "expectedBaseDeliverySetHash",
                "expectedPointerSnapshotToken",
                "pointerBytesBase64",
                "deliverySetPath",
                "deliverySetBytesBase64",
                "deliverySetSnapshotToken",
            ],
        )?;
        if self.read_only {
            return Err(CustodyError::input(
                "publishPointer is forbidden in a read-only custody session",
            ));
        }
        self.require_lock()?;
        let path = required_safe_relative_path(object, "path")?;
        if path != "delivery-pointer-v2.json" {
            return Err(CustodyError::input(
                "pointer CAS may only publish delivery-pointer-v2.json",
            ));
        }
        let expected_base = required_string(object, "expectedBaseDeliverySetHash")?;
        validate_base_delivery_set_hash(&expected_base)?;
        let expected_token = required_string(object, "expectedPointerSnapshotToken")?;
        validate_snapshot_token(&expected_token)?;
        let pointer_encoded = required_string(object, "pointerBytesBase64")?;
        let bytes = decode_base64url_bounded(&pointer_encoded, MAX_POINTER_BYTES)?;
        let delivery_set_path = required_string(object, "deliverySetPath")?;
        validate_safe_relative_path(&delivery_set_path)?;
        let delivery_set_encoded = required_string(object, "deliverySetBytesBase64")?;
        let delivery_set_bytes =
            decode_base64url_bounded(&delivery_set_encoded, MAX_POINTER_BYTES)?;
        let delivery_set_token = required_string(object, "deliverySetSnapshotToken")?;
        if delivery_set_token == "NONE" {
            return Err(CustodyError::input(
                "deliverySetSnapshotToken must identify a present delivery set",
            ));
        }
        validate_snapshot_token(&delivery_set_token)?;
        let narrative_id = self
            .narrative_id
            .as_deref()
            .ok_or_else(|| CustodyError::input("narrative is not initialized"))?;
        let pointer = parse_pointer_record(&bytes, narrative_id)?;
        if pointer.base_delivery_set_hash != expected_base {
            return Err(CustodyError::new(
                "trackb-cas-conflict",
                "pointer baseDeliverySetHash does not match expected CAS base",
                false,
            ));
        }
        let delivery_set_relative = pointer_delivery_set_relative(&pointer, narrative_id)?;
        if delivery_set_path != expected_delivery_set_project_path(&pointer, narrative_id)? {
            return Err(CustodyError::storage(
                "deliverySetPath is not the exact pointer-referenced project path",
            ));
        }
        let (delivery_set_parent, delivery_set_name) =
            self.storage_parent(&delivery_set_relative, false)?;
        let supplied_delivery_set =
            snapshot_entry(&delivery_set_parent, &delivery_set_name, MAX_POINTER_BYTES)?;
        if !supplied_delivery_set.exists
            || supplied_delivery_set.bytes != delivery_set_bytes
            || supplied_delivery_set.token() != delivery_set_token
        {
            return Err(CustodyError::storage(
                "referenced delivery set bytes or snapshot identity changed",
            ));
        }
        let delivery_set = parse_delivery_set_record(&delivery_set_bytes, narrative_id)?;
        validate_pointer_delivery_set(&pointer, &delivery_set)?;
        let (parent, target_name) = self.storage_parent(&path, false)?;
        let current = snapshot_entry(&parent, &target_name, MAX_POINTER_BYTES)?;
        if !current.exists {
            if expected_base != "NONE" || expected_token != "NONE" {
                return Err(CustodyError::new(
                    "trackb-cas-conflict",
                    "pointer is absent but the expected CAS base is not NONE",
                    false,
                ));
            }
        } else {
            let current_pointer = parse_pointer_record(&current.bytes, narrative_id)?;
            let current_delivery_set_relative =
                pointer_delivery_set_relative(&current_pointer, narrative_id)?;
            let (current_delivery_set_parent, current_delivery_set_name) =
                self.storage_parent(&current_delivery_set_relative, false)?;
            let current_delivery_set_snapshot = snapshot_entry(
                &current_delivery_set_parent,
                &current_delivery_set_name,
                MAX_POINTER_BYTES,
            )?;
            if !current_delivery_set_snapshot.exists {
                return Err(CustodyError::storage(
                    "current pointer references an absent delivery set",
                ));
            }
            let current_delivery_set =
                parse_delivery_set_record(&current_delivery_set_snapshot.bytes, narrative_id)?;
            validate_pointer_delivery_set(&current_pointer, &current_delivery_set)?;
            if expected_token != current.token()
                || expected_base != current_pointer.delivery_set_hash()
            {
                return Err(CustodyError::new(
                    "trackb-cas-conflict",
                    "pointer CAS base or snapshot no longer matches",
                    false,
                ));
            }
        }

        self.temporary_counter = self.temporary_counter.saturating_add(1);
        let operation_id = self
            .operation_id
            .as_deref()
            .ok_or_else(|| CustodyError::input("operation is not initialized"))?;
        let temporary_name = format!(
            ".delivery-pointer-v2.{operation_id}.{}.tmp",
            self.temporary_counter
        );
        let mut temporary = open_exclusive_at(&parent, &temporary_name, 0o600)
            .map_err(|error| CustodyError::io("pointer temporary cannot be created", &error))?;
        let mut temporary_identity = file_identity(&temporary)
            .map_err(|error| CustodyError::io("pointer temporary identity failed", &error))?;
        let operation_result = (|| -> Result<Value> {
            temporary
                .write_all(&bytes)
                .map_err(|error| CustodyError::io("pointer temporary write failed", &error))?;
            temporary
                .sync_all()
                .map_err(|error| CustodyError::io("pointer temporary fsync failed", &error))?;
            temporary_identity = file_identity(&temporary).map_err(|error| {
                CustodyError::io("pointer temporary identity failed after write", &error)
            })?;
            #[cfg(test)]
            apply_publish_test_fault(&parent, &temporary_name, fault.as_ref())?;
            let before_rename = snapshot_entry(&parent, &target_name, MAX_POINTER_BYTES)?;
            let before_state = if before_rename.exists {
                let before_pointer = parse_pointer_record(&before_rename.bytes, narrative_id)?;
                let before_delivery_set_relative =
                    pointer_delivery_set_relative(&before_pointer, narrative_id)?;
                let (before_delivery_set_parent, before_delivery_set_name) =
                    self.storage_parent(&before_delivery_set_relative, false)?;
                let before_delivery_set_snapshot = snapshot_entry(
                    &before_delivery_set_parent,
                    &before_delivery_set_name,
                    MAX_POINTER_BYTES,
                )?;
                if !before_delivery_set_snapshot.exists {
                    return Err(CustodyError::storage(
                        "pointer delivery set disappeared before CAS rename",
                    ));
                }
                let before_delivery_set =
                    parse_delivery_set_record(&before_delivery_set_snapshot.bytes, narrative_id)?;
                validate_pointer_delivery_set(&before_pointer, &before_delivery_set)?;
                Some((before_pointer, before_rename.token()))
            } else {
                None
            };
            let expected_matches = match before_state {
                None => expected_base == "NONE" && expected_token == "NONE",
                Some((before_pointer, token)) => {
                    expected_base == before_pointer.delivery_set_hash() && expected_token == token
                }
            };
            if !expected_matches {
                return Err(CustodyError::new(
                    "trackb-cas-conflict",
                    "pointer changed before CAS rename",
                    false,
                ));
            }
            assert_owned_temporary(&parent, &temporary_name, temporary_identity)?;
            #[cfg(test)]
            if matches!(fault.as_ref(), Some(PublishTestFault::FailBeforeRename)) {
                return Err(CustodyError::storage(
                    "test seam injected a pre-rename pointer failure",
                ));
            }
            rename_at(&parent, &temporary_name, &target_name)
                .map_err(|error| CustodyError::io("pointer CAS rename failed", &error))?;
            #[cfg(test)]
            if matches!(
                fault.as_ref(),
                Some(PublishTestFault::FailAfterRenameBeforeSync)
            ) {
                return Err(CustodyError::new(
                    "trackb-pointer-reconcile-required",
                    "test seam injected post-rename fsync uncertainty",
                    false,
                ));
            }
            if let Err(error) = sync_directory(&parent) {
                return Err(CustodyError::new(
                    "trackb-pointer-reconcile-required",
                    format!("pointer renamed but parent durability is unknown: {error}"),
                    false,
                ));
            }
            #[cfg(test)]
            if matches!(
                fault.as_ref(),
                Some(PublishTestFault::FailAfterSyncBeforeReopen)
            ) {
                return Err(CustodyError::new(
                    "trackb-pointer-reconcile-required",
                    "test seam injected post-rename reopen uncertainty",
                    false,
                ));
            }
            let published =
                snapshot_entry(&parent, &target_name, MAX_POINTER_BYTES).map_err(|error| {
                    CustodyError::new(
                        "trackb-pointer-reconcile-required",
                        format!(
                            "pointer renamed but published bytes cannot be verified: {}",
                            error.message
                        ),
                        false,
                    )
                })?;
            if !published.exists || published.bytes != bytes {
                return Err(CustodyError::new(
                    "trackb-pointer-reconcile-required",
                    "pointer renamed but published bytes differ from the requested bytes",
                    false,
                ));
            }
            let reopened_pointer =
                parse_pointer_record(&published.bytes, narrative_id).map_err(|error| {
                    CustodyError::new(
                        "trackb-pointer-reconcile-required",
                        format!(
                            "published pointer failed strict validation: {}",
                            error.message
                        ),
                        false,
                    )
                })?;
            let reopened_delivery_set_relative =
                pointer_delivery_set_relative(&reopened_pointer, narrative_id).map_err(
                    |error| {
                        CustodyError::new(
                            "trackb-pointer-reconcile-required",
                            format!(
                                "published pointer delivery path is invalid: {}",
                                error.message
                            ),
                            false,
                        )
                    },
                )?;
            let (reopened_delivery_set_parent, reopened_delivery_set_name) = self
                .storage_parent(&reopened_delivery_set_relative, false)
                .map_err(|error| {
                    CustodyError::new(
                        "trackb-pointer-reconcile-required",
                        format!(
                            "published pointer delivery set cannot be reopened: {}",
                            error.message
                        ),
                        false,
                    )
                })?;
            let reopened_delivery_set_snapshot = snapshot_entry(
                &reopened_delivery_set_parent,
                &reopened_delivery_set_name,
                MAX_POINTER_BYTES,
            )
            .map_err(|error| {
                CustodyError::new(
                    "trackb-pointer-reconcile-required",
                    format!(
                        "published delivery set cannot be verified: {}",
                        error.message
                    ),
                    false,
                )
            })?;
            if !reopened_delivery_set_snapshot.exists {
                return Err(CustodyError::new(
                    "trackb-pointer-reconcile-required",
                    "published pointer references an absent delivery set",
                    false,
                ));
            }
            let reopened_delivery_set =
                parse_delivery_set_record(&reopened_delivery_set_snapshot.bytes, narrative_id)
                    .map_err(|error| {
                        CustodyError::new(
                            "trackb-pointer-reconcile-required",
                            format!(
                                "published delivery set failed strict validation: {}",
                                error.message
                            ),
                            false,
                        )
                    })?;
            validate_pointer_delivery_set(&reopened_pointer, &reopened_delivery_set).map_err(
                |error| {
                    CustodyError::new(
                        "trackb-pointer-reconcile-required",
                        format!(
                            "published pointer/delivery set binding failed: {}",
                            error.message
                        ),
                        false,
                    )
                },
            )?;
            Ok(json_object([
                ("path", Value::String(path.clone())),
                ("reused", Value::Bool(false)),
                ("snapshot", snapshot_value("", &published)),
            ]))
        })();
        if let Err(error) = &operation_result {
            // After rename the temp entry no longer exists.  Before rename,
            // cleanup proves our exact inode and never scans the directory.
            #[cfg(test)]
            let cleanup_result = cleanup_owned_temp_with_test_fault(
                &parent,
                &temporary_name,
                temporary_identity,
                fault.as_ref(),
            );
            #[cfg(not(test))]
            let cleanup_result = cleanup_owned_temp(&parent, &temporary_name, temporary_identity);
            if let Err(cleanup_error) = cleanup_result {
                return Err(CustodyError::storage(format!(
                    "pointer temporary cleanup failed after {}: {}",
                    error.code, cleanup_error.message
                )));
            }
            return Err(error.clone());
        }
        operation_result
    }
}
