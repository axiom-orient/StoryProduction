//! Owned temporary publication and cleanup, including test-only fault seams.

#[cfg(test)]
use super::descriptors::{open_exclusive_at, symlink_at, unlink_at};
use super::descriptors::{sync_directory, unlink_exact};
use crate::error::{CustodyError, Result};
use crate::filesystem::descriptors::{
    fstatat_no_follow, stat_identity, stat_is_regular, FileIdentity,
};
use std::fs::File;
use std::io;

#[cfg(test)]
use std::io::Write;

pub(crate) fn cleanup_owned_temp(parent: &File, name: &str, expected: FileIdentity) -> Result<()> {
    #[cfg(test)]
    {
        cleanup_owned_temp_impl(parent, name, expected, None)
    }
    #[cfg(not(test))]
    {
        cleanup_owned_temp_impl(parent, name, expected)
    }
}

#[cfg(test)]
pub(crate) fn cleanup_owned_temp_with_test_fault(
    parent: &File,
    name: &str,
    expected: FileIdentity,
    fault: Option<&PublishTestFault>,
) -> Result<()> {
    cleanup_owned_temp_impl(parent, name, expected, fault)
}

pub(crate) fn cleanup_owned_temp_impl(
    parent: &File,
    name: &str,
    expected: FileIdentity,
    #[cfg(test)] fault: Option<&PublishTestFault>,
) -> Result<()> {
    let stat = match fstatat_no_follow(parent, name) {
        Ok(stat) => stat,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(CustodyError::storage(format!(
                "temporary identity could not be verified: {error}"
            )))
        }
    };
    if !stat_is_regular(&stat) || stat_identity(&stat) != expected {
        return Err(CustodyError::storage(
            "temporary entry identity changed; residue preserved",
        ));
    }
    #[cfg(test)]
    apply_cleanup_test_fault(parent, name, fault)?;
    unlink_exact(parent, name, expected)
        .map_err(|error| CustodyError::storage(format!("temporary unlink failed: {error}")))?;
    sync_directory(parent).map_err(|error| {
        CustodyError::storage(format!("temporary parent fsync failed: {error}"))
    })?;
    Ok(())
}

pub(crate) fn assert_owned_temporary(
    parent: &File,
    name: &str,
    expected: FileIdentity,
) -> Result<()> {
    let stat = fstatat_no_follow(parent, name).map_err(|error| {
        CustodyError::storage(format!(
            "temporary identity cannot be verified before rename: {error}"
        ))
    })?;
    if !stat_is_regular(&stat) || stat_identity(&stat) != expected {
        return Err(CustodyError::storage(
            "temporary entry was replaced before rename; residue preserved",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PublishTestFault {
    None,
    FailBeforeRename,
    ReplaceTempRegular,
    ReplaceTempSymlink,
    ReplaceTempAfterCleanupIdentity,
    ReplaceTempSymlinkAfterCleanupIdentity,
    FailAfterRenameBeforeSync,
    FailAfterSyncBeforeReopen,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum MeasurementPublishTestFault {
    None,
    FailParentSync,
    FailBeforeReopen,
}

#[cfg(test)]
pub(crate) fn apply_publish_test_fault(
    parent: &File,
    temporary_name: &str,
    fault: Option<&PublishTestFault>,
) -> Result<()> {
    match fault {
        Some(PublishTestFault::ReplaceTempRegular) => {
            unlink_at(parent, temporary_name).map_err(|error| {
                CustodyError::storage(format!("test temporary replacement failed: {error}"))
            })?;
            let mut replacement =
                open_exclusive_at(parent, temporary_name, 0o600).map_err(|error| {
                    CustodyError::storage(format!(
                        "test temporary replacement create failed: {error}"
                    ))
                })?;
            replacement.write_all(b"replacement").map_err(|error| {
                CustodyError::storage(format!("test temporary replacement write failed: {error}"))
            })?;
            replacement.sync_all().map_err(|error| {
                CustodyError::storage(format!("test temporary replacement fsync failed: {error}"))
            })?;
        }
        Some(PublishTestFault::ReplaceTempSymlink) => {
            unlink_at(parent, temporary_name).map_err(|error| {
                CustodyError::storage(format!("test temporary replacement failed: {error}"))
            })?;
            let target_name = "test-temporary-target";
            let mut target = open_exclusive_at(parent, target_name, 0o600).map_err(|error| {
                CustodyError::storage(format!("test temporary target create failed: {error}"))
            })?;
            target.write_all(b"replacement-target").map_err(|error| {
                CustodyError::storage(format!("test temporary target write failed: {error}"))
            })?;
            target.sync_all().map_err(|error| {
                CustodyError::storage(format!("test temporary target fsync failed: {error}"))
            })?;
            symlink_at(parent, target_name, temporary_name).map_err(|error| {
                CustodyError::storage(format!(
                    "test temporary symlink replacement failed: {error}"
                ))
            })?;
        }
        Some(PublishTestFault::ReplaceTempAfterCleanupIdentity)
        | Some(PublishTestFault::ReplaceTempSymlinkAfterCleanupIdentity) => {
            return Err(CustodyError::storage(
                "test seam injects temporary replacement after cleanup identity capture",
            ));
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn apply_cleanup_test_fault(
    parent: &File,
    temporary_name: &str,
    fault: Option<&PublishTestFault>,
) -> Result<()> {
    let replace = match fault {
        Some(PublishTestFault::ReplaceTempAfterCleanupIdentity) => Some(false),
        Some(PublishTestFault::ReplaceTempSymlinkAfterCleanupIdentity) => Some(true),
        _ => None,
    };
    let Some(symlink) = replace else {
        return Ok(());
    };
    unlink_at(parent, temporary_name).map_err(|error| {
        CustodyError::storage(format!("test temporary replacement failed: {error}"))
    })?;
    if symlink {
        let target_name = "test-cleanup-temporary-target";
        let mut target = open_exclusive_at(parent, target_name, 0o600).map_err(|error| {
            CustodyError::storage(format!("test cleanup target create failed: {error}"))
        })?;
        target
            .write_all(b"cleanup-replacement-target")
            .map_err(|error| {
                CustodyError::storage(format!("test cleanup target write failed: {error}"))
            })?;
        target.sync_all().map_err(|error| {
            CustodyError::storage(format!("test cleanup target fsync failed: {error}"))
        })?;
        symlink_at(parent, target_name, temporary_name).map_err(|error| {
            CustodyError::storage(format!("test cleanup symlink replacement failed: {error}"))
        })?;
    } else {
        let mut replacement =
            open_exclusive_at(parent, temporary_name, 0o600).map_err(|error| {
                CustodyError::storage(format!("test cleanup replacement create failed: {error}"))
            })?;
        replacement
            .write_all(b"cleanup-replacement")
            .map_err(|error| {
                CustodyError::storage(format!("test cleanup replacement write failed: {error}"))
            })?;
        replacement.sync_all().map_err(|error| {
            CustodyError::storage(format!("test cleanup replacement fsync failed: {error}"))
        })?;
    }
    Ok(())
}
