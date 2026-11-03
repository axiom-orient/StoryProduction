//! Hash-bound reads of persisted DeliverySets for the current narrative release.

use super::{
    load_contract_set, project_owned_path, verify_delivery_artifacts, verify_released_package,
    DeliveryEngine, DeliveryRecord,
};
use narrative_courier_contracts::{ArtifactRef, DeliverySet, NarrativePackage};
use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::{Error, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

impl DeliveryEngine {
    pub fn load(
        project: &Path,
        package: &NarrativePackage,
        request_id: &str,
    ) -> Result<DeliverySet> {
        let package_sha = verify_released_package(project, package)?;
        let set = load_contract_set(project, package, request_id)?;
        if set.narrative_package_sha256 != package_sha {
            return Err(Error::Contract(
                "STALE_RESULT: delivery does not target current released NarrativePackage".into(),
            ));
        }
        verify_delivery_artifacts(project, &set)?;
        Ok(set)
    }

    /// Read one current Delivery artifact and return the exact bytes whose digest was verified.
    ///
    /// `max_bytes` bounds the captured preview/read buffer. The artifact path and persisted
    /// contract chain remain Delivery-owned; callers must use the returned bytes instead of
    /// reopening the artifact path themselves.
    pub fn read_artifact_bytes(
        project: &Path,
        package: &NarrativePackage,
        request_id: &str,
        output_id: &str,
        max_bytes: u64,
    ) -> Result<(ArtifactRef, Vec<u8>)> {
        let set = Self::load(project, package, request_id)?;
        let receipt = set
            .deliveries
            .iter()
            .find(|delivery| delivery.output_request_id == output_id)
            .ok_or_else(|| Error::Contract("delivery output not found".into()))?;
        let artifact = &receipt.artifact;
        let bytes = capture_verified_artifact(project, artifact, max_bytes)?;
        Ok((artifact.clone(), bytes))
    }

    pub fn list(project: &Path, package: &NarrativePackage) -> Result<Vec<DeliveryRecord>> {
        let package_sha = verify_released_package(project, package)?;
        let root = project_owned_path(project, "visual/requests")?;
        match fs::symlink_metadata(&root) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Ok(_) => {
                return Err(Error::Contract(
                    "delivery request root is not a regular directory".into(),
                ))
            }
            Err(error) => return Err(error.into()),
        }
        let mut records = Vec::new();
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let request_id = entry.file_name().to_string_lossy().into_owned();
            if validate_path_segment(&request_id, "request_id").is_err() {
                continue;
            }
            let relative = format!("visual/requests/{request_id}/delivery-set.json");
            let path = project_owned_path(project, &relative)?;
            if path.is_file() {
                let set = match load_contract_set(project, package, &request_id) {
                    Ok(set) => set,
                    Err(Error::Contract(message)) if message.starts_with("STALE_RESULT:") => {
                        continue
                    }
                    Err(error) => return Err(error),
                };
                if set.narrative_package_sha256 != package_sha {
                    continue;
                }
                verify_delivery_artifacts(project, &set)?;
                records.push(DeliveryRecord { request_id, set });
            }
        }
        records.sort_by(|a, b| a.request_id.cmp(&b.request_id));
        Ok(records)
    }
}

fn capture_verified_artifact(
    project: &Path,
    artifact: &ArtifactRef,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    let path = project_owned_path(project, &artifact.path)?;
    let path_metadata = fs::symlink_metadata(&path)?;
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        return Err(Error::Contract(format!(
            "INVALID_PATH: delivery artifact is not a regular file: {}",
            artifact.path
        )));
    }
    if path_metadata.len() > max_bytes {
        return Err(Error::Contract(format!(
            "delivery artifact exceeds {max_bytes} byte read limit"
        )));
    }

    let mut file = File::open(&path)?;
    let opened_metadata = file.metadata()?;
    if !opened_metadata.is_file() || opened_metadata.len() > max_bytes {
        return Err(Error::Contract(format!(
            "INVALID_PATH: delivery artifact changed before read: {}",
            artifact.path
        )));
    }
    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    file.by_ref()
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(Error::Contract(format!(
            "delivery artifact exceeds {max_bytes} byte read limit"
        )));
    }
    if sha256_bytes(&bytes) != artifact.sha256 {
        return Err(Error::Contract(format!(
            "HASH_MISMATCH: delivery artifact changed while reading: {}",
            artifact.path
        )));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::capture_verified_artifact;
    use narrative_courier_contracts::ArtifactRef;
    use narrative_courier_runtime::store::sha256_bytes;
    use std::fs;
    use tempfile::TempDir;

    fn artifact(path: &str, bytes: &[u8]) -> ArtifactRef {
        ArtifactRef {
            kind: "delivery_output".into(),
            path: path.into(),
            sha256: sha256_bytes(bytes),
            mime: Some("text/plain".into()),
        }
    }

    #[test]
    fn captured_delivery_bytes_are_bounded_and_hash_bound() {
        let temporary = TempDir::new().expect("temporary root");
        let project = temporary.path().join("project");
        fs::create_dir_all(project.join("visual/outputs")).expect("project output directory");
        let path = project.join("visual/outputs/preview.txt");
        let original = b"verified preview";
        fs::write(&path, original).expect("write artifact");
        let reference = artifact("visual/outputs/preview.txt", original);

        assert_eq!(
            capture_verified_artifact(&project, &reference, 64).expect("verified bytes"),
            original.to_vec()
        );
        assert!(capture_verified_artifact(&project, &reference, 4)
            .expect_err("oversized preview must fail")
            .to_string()
            .contains("read limit"));

        fs::write(&path, b"changed after receipt").expect("replace artifact bytes");
        assert!(capture_verified_artifact(&project, &reference, 64)
            .expect_err("changed bytes must fail")
            .to_string()
            .contains("HASH_MISMATCH"));
    }
}
