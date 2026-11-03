//! Lock acquisition, stale-owner proof, and exact lock release.

use std::fs::File;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::artifacts::snapshot_entry;
use crate::error::{CustodyError, Result};
use crate::filesystem::descriptors::{
    file_identity, open_exclusive_at, sync_directory, unlink_exact, FileIdentity,
};
use crate::process_identity::{current_process_start_identity, probe_owner, OwnerProbe};
use crate::protocol::{canonical_json, is_utc_timestamp, sha256_prefixed, utc_now};
use crate::record_fields::has_exact_value_keys;
const MAX_LOCK_BYTES: usize = 16 * 1024;
const LOCK_PROBE_COUNT: usize = 3;
const LOCK_PROBE_INTERVAL_MS: u64 = 250;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LockRecord {
    schema_version: u64,
    narrative_id: String,
    operation_id: String,
    owner_pid: u32,
    process_start_identity: String,
    created_at: String,
    lock_hash: String,
}

#[derive(Debug)]
pub(super) struct LockHandle {
    file: File,
    /// Keep the admitted narrative directory alive through release and stale-owner checks.
    parent: File,
    identity: FileIdentity,
    record: LockRecord,
    record_bytes: Vec<u8>,
    owner_pid: u32,
    owner_process_start_identity: String,
    owner_operation_id: String,
}

pub(super) fn acquire_lock(
    parent: File,
    narrative_id: &str,
    operation_id: &str,
) -> Result<LockHandle> {
    let lock_name = ".release.lock";
    for attempt in 0..=1 {
        let record = new_lock_record(narrative_id, operation_id)?;
        match open_exclusive_at(&parent, lock_name, 0o600) {
            Ok(mut file) => {
                let bytes = lock_record_bytes(&record)?;
                file.write_all(&bytes)
                    .map_err(|error| CustodyError::io("lock record write failed", &error))?;
                file.sync_all()
                    .map_err(|error| CustodyError::io("lock record fsync failed", &error))?;
                let identity = file_identity(&file)
                    .map_err(|error| CustodyError::io("lock inode identity failed", &error))?;
                sync_directory(&parent)
                    .map_err(|error| CustodyError::io("lock parent fsync failed", &error))?;
                return Ok(LockHandle {
                    file,
                    parent,
                    identity,
                    owner_pid: record.owner_pid,
                    owner_process_start_identity: record.process_start_identity.clone(),
                    owner_operation_id: record.operation_id.clone(),
                    record,
                    record_bytes: bytes,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let initial_snapshot =
                    snapshot_entry(&parent, lock_name, MAX_LOCK_BYTES).map_err(|error| {
                        CustodyError::recovery(format!(
                            "existing lock cannot be inspected safely: {}",
                            error.message
                        ))
                    })?;
                if !initial_snapshot.exists {
                    return Err(CustodyError::recovery(
                        "existing lock disappeared during stale inspection",
                    ));
                }
                let (existing, initial_bytes) =
                    parse_lock_record(&initial_snapshot.bytes, narrative_id)?;
                let initial = initial_snapshot
                    .identity
                    .expect("present lock snapshot has an identity");
                let mut saw_live = false;
                for probe in 0..LOCK_PROBE_COUNT {
                    match probe_owner(existing.owner_pid, &existing.process_start_identity) {
                        OwnerProbe::Live => {
                            saw_live = true;
                            if probe + 1 < LOCK_PROBE_COUNT {
                                thread::sleep(Duration::from_millis(LOCK_PROBE_INTERVAL_MS));
                            }
                        }
                        OwnerProbe::Dead | OwnerProbe::Reused => break,
                        OwnerProbe::Unknown => {
                            return Err(CustodyError::recovery(
                                "lock owner liveness or process start identity is unknown",
                            ));
                        }
                    }
                }
                if saw_live {
                    return Err(CustodyError::new(
                        "trackb-lock-busy",
                        "narrative release lock is held by a live owner",
                        true,
                    ));
                }
                let final_snapshot =
                    snapshot_entry(&parent, lock_name, MAX_LOCK_BYTES).map_err(|error| {
                        CustodyError::recovery(format!(
                            "lock bytes changed during stale proof: {}",
                            error.message
                        ))
                    })?;
                if !final_snapshot.exists
                    || final_snapshot.identity != Some(initial)
                    || final_snapshot.bytes != initial_bytes
                {
                    return Err(CustodyError::recovery(
                        "lock inode or record bytes changed while proving stale ownership",
                    ));
                }
                unlink_exact(&parent, lock_name, initial).map_err(|error| {
                    CustodyError::recovery(format!(
                        "stale lock reclaim could not prove ownership: {error}"
                    ))
                })?;
                sync_directory(&parent)
                    .map_err(|error| CustodyError::io("stale lock parent fsync failed", &error))?;
                if attempt == 1 {
                    return Err(CustodyError::recovery(
                        "stale lock reclaim did not leave a clean acquisition boundary",
                    ));
                }
            }
            Err(error) => {
                return Err(CustodyError::io("lock creation failed", &error));
            }
        }
    }
    Err(CustodyError::recovery("lock acquisition did not converge"))
}

pub(super) fn release_lock(lock: LockHandle) -> Result<()> {
    let fd_identity = file_identity(&lock.file)
        .map_err(|error| CustodyError::io("lock release fd identity failed", &error))?;
    let current =
        snapshot_entry(&lock.parent, ".release.lock", MAX_LOCK_BYTES).map_err(|error| {
            CustodyError::recovery(format!(
                "lock release path cannot be verified: {}",
                error.message
            ))
        })?;
    if fd_identity != lock.identity || !current.exists || current.identity != Some(lock.identity) {
        return Err(CustodyError::recovery(
            "lock release refused after exact inode identity changed",
        ));
    }
    if current.bytes != lock.record_bytes {
        return Err(CustodyError::recovery(
            "lock release refused after canonical record or inode identity changed",
        ));
    }
    let (record, _) =
        read_lock_record_at(&lock.parent, ".release.lock", &lock.record.narrative_id)?;
    if record.owner_pid != lock.owner_pid
        || record.process_start_identity != lock.owner_process_start_identity
        || record.operation_id != lock.owner_operation_id
        || record.narrative_id != lock.record.narrative_id
        || record.lock_hash != lock.record.lock_hash
    {
        return Err(CustodyError::recovery(
            "lock release refused after owner tuple changed",
        ));
    }
    unlink_exact(&lock.parent, ".release.lock", lock.identity)
        .map_err(|error| CustodyError::recovery(format!("lock release unlink failed: {error}")))?;
    sync_directory(&lock.parent)
        .map_err(|error| CustodyError::io("lock release parent fsync failed", &error))?;
    Ok(())
}

fn new_lock_record(narrative_id: &str, operation_id: &str) -> Result<LockRecord> {
    let owner_pid = std::process::id();
    let process_start_identity = current_process_start_identity();
    let mut record = LockRecord {
        schema_version: 1,
        narrative_id: narrative_id.to_owned(),
        operation_id: operation_id.to_owned(),
        owner_pid,
        process_start_identity,
        created_at: utc_now(),
        lock_hash: String::new(),
    };
    record.lock_hash = lock_record_hash(&record)?;
    Ok(record)
}

fn lock_record_hash(record: &LockRecord) -> Result<String> {
    let mut object = lock_record_value(record);
    object.remove("lockHash");
    Ok(sha256_prefixed(
        canonical_json(&Value::Object(object)).as_bytes(),
    ))
}

fn lock_record_bytes(record: &LockRecord) -> Result<Vec<u8>> {
    let value = Value::Object(lock_record_value(record));
    let bytes = canonical_json(&value).into_bytes();
    if bytes.len() > MAX_LOCK_BYTES {
        return Err(CustodyError::storage("lock record exceeds bounded size"));
    }
    Ok(bytes)
}

fn lock_record_value(record: &LockRecord) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert(
        "createdAt".to_owned(),
        Value::String(record.created_at.clone()),
    );
    object.insert(
        "lockHash".to_owned(),
        Value::String(record.lock_hash.clone()),
    );
    object.insert(
        "narrativeId".to_owned(),
        Value::String(record.narrative_id.clone()),
    );
    object.insert(
        "operationId".to_owned(),
        Value::String(record.operation_id.clone()),
    );
    object.insert("ownerPid".to_owned(), Value::from(record.owner_pid));
    object.insert(
        "processStartIdentity".to_owned(),
        Value::String(record.process_start_identity.clone()),
    );
    object.insert(
        "schemaVersion".to_owned(),
        Value::from(record.schema_version),
    );
    object
}

fn read_lock_record_at(
    parent: &File,
    name: &str,
    narrative_id: &str,
) -> Result<(LockRecord, Vec<u8>)> {
    let snapshot = snapshot_entry(parent, name, MAX_LOCK_BYTES)?;
    if !snapshot.exists {
        return Err(CustodyError::recovery(
            "lock disappeared during stale inspection",
        ));
    }
    parse_lock_record(&snapshot.bytes, narrative_id)
}

fn parse_lock_record(bytes: &[u8], narrative_id: &str) -> Result<(LockRecord, Vec<u8>)> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| CustodyError::recovery(format!("lock JSON is malformed: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| CustodyError::recovery("lock record must be a JSON object"))?;
    if !has_exact_value_keys(
        object,
        &[
            "schemaVersion",
            "narrativeId",
            "operationId",
            "ownerPid",
            "processStartIdentity",
            "createdAt",
            "lockHash",
        ],
    ) {
        return Err(CustodyError::recovery(
            "lock record fields are not the exact v1 set",
        ));
    }
    let record: LockRecord = serde_json::from_value(value).map_err(|error| {
        CustodyError::recovery(format!("lock record fields are invalid: {error}"))
    })?;
    if record.schema_version != 1
        || record.narrative_id != narrative_id
        || record.owner_pid == 0
        || record.owner_pid > i32::MAX as u32
        || record.process_start_identity.trim().is_empty()
        || record.process_start_identity.len() > 2048
        || record.operation_id.is_empty()
        || !is_utc_timestamp(&record.created_at)
        || record.lock_hash != lock_record_hash(&record)?
    {
        return Err(CustodyError::recovery(
            "lock record schema, owner, timestamp, narrative, or hash is invalid",
        ));
    }
    let canonical_bytes = lock_record_bytes(&record)?;
    if bytes != canonical_bytes {
        return Err(CustodyError::recovery(
            "lock record bytes are not the canonical v1 representation",
        ));
    }
    Ok((record, canonical_bytes))
}
