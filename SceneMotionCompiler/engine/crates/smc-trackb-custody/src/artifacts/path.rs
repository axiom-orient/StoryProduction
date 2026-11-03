//! Descriptor-relative artifact path walks and retained directory-edge proofs.

use std::fs::File;
use std::io;

use crate::artifacts::{snapshot_entry, Snapshot};
use crate::error::{CustodyError, Result};
use crate::filesystem::admission::validate_safe_relative_path;
use crate::filesystem::descriptors::{
    directory_identity, ensure_directory_at, fstatat_no_follow, open_directory_at, stat_identity,
    stat_is_directory, FileIdentity,
};

#[derive(Debug)]
pub(crate) struct DirectoryEdge {
    parent: File,
    name: String,
    identity: FileIdentity,
}

pub(crate) fn read_project_artifact_at(
    project: &File,
    path: &str,
    maximum_bytes: usize,
) -> Result<Snapshot> {
    validate_safe_relative_path(path)?;
    let components: Vec<&str> = path.split('/').collect();
    let name = components
        .last()
        .copied()
        .ok_or_else(|| CustodyError::storage("project artifact path has no file name"))?;
    let mut directory = project
        .try_clone()
        .map_err(|error| CustodyError::io("project root descriptor cannot be cloned", &error))?;
    let mut edges = Vec::with_capacity(components.len().saturating_sub(1));
    for component in &components[..components.len() - 1] {
        let initial_stat = match fstatat_no_follow(&directory, component) {
            Ok(stat) => stat,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                revalidate_project_directory_edges(&edges)?;
                return Ok(Snapshot::absent());
            }
            Err(error) => {
                return Err(CustodyError::storage(format!(
                    "project artifact parent cannot be inspected safely: {error}"
                )))
            }
        };
        let (opened, edge) = open_directory_edge(
            &directory,
            component,
            initial_stat,
            "project artifact parent",
            "read",
        )?;
        edges.push(edge);
        directory = opened;
    }
    let snapshot = snapshot_entry(&directory, name, maximum_bytes)?;
    revalidate_project_directory_edges(&edges)?;
    Ok(snapshot)
}

pub(crate) fn revalidate_project_directory_edges(edges: &[DirectoryEdge]) -> Result<()> {
    for edge in edges.iter().rev() {
        let current = fstatat_no_follow(&edge.parent, &edge.name).map_err(|error| {
            CustodyError::storage(format!(
                "project artifact directory edge cannot be revalidated: {error}"
            ))
        })?;
        if !stat_is_directory(&current) || stat_identity(&current) != edge.identity {
            return Err(CustodyError::storage(
                "project artifact directory edge was replaced while being read",
            ));
        }
    }
    Ok(())
}

fn open_directory_edge(
    parent: &File,
    name: &str,
    initial_stat: libc::stat,
    label: &str,
    preposition: &str,
) -> Result<(File, DirectoryEdge)> {
    if !stat_is_directory(&initial_stat) {
        return Err(CustodyError::storage(format!(
            "{label} must be a regular non-symlink directory"
        )));
    }
    let opened = open_directory_at(parent, name).map_err(|error| {
        CustodyError::storage(format!("{label} changed while being opened: {error}"))
    })?;
    let opened_identity = directory_identity(&opened).map_err(|error| {
        CustodyError::io(&format!("{label} identity cannot be verified"), &error)
    })?;
    let expected_identity = stat_identity(&initial_stat);
    if opened_identity != expected_identity {
        return Err(CustodyError::storage(format!(
            "{label} changed before it was {preposition}"
        )));
    }
    let edge_parent = parent.try_clone().map_err(|error| {
        CustodyError::io(&format!("{label} descriptor cannot be cloned"), &error)
    })?;
    let edge = DirectoryEdge {
        parent: edge_parent,
        name: name.to_owned(),
        identity: expected_identity,
    };
    Ok((opened, edge))
}

pub(crate) fn measurement_storage_parent(
    project: &File,
    path: &str,
    create_parent: bool,
) -> Result<(File, String, Vec<DirectoryEdge>)> {
    validate_safe_relative_path(path)?;
    let components: Vec<&str> = path.split('/').collect();
    let name = components
        .last()
        .copied()
        .ok_or_else(|| CustodyError::storage("measurement receipt path has no file name"))?;
    let mut directory = project
        .try_clone()
        .map_err(|error| CustodyError::io("project root descriptor cannot be cloned", &error))?;
    let mut edges = Vec::with_capacity(components.len().saturating_sub(1));
    for component in &components[..components.len() - 1] {
        let initial_stat = match fstatat_no_follow(&directory, component) {
            Ok(stat) => stat,
            Err(error) if error.kind() == io::ErrorKind::NotFound && create_parent => {
                ensure_directory_at(&directory, component).map_err(|error| {
                    CustodyError::io(
                        "measurement receipt parent cannot be created safely",
                        &error,
                    )
                })?;
                fstatat_no_follow(&directory, component).map_err(|error| {
                    CustodyError::io(
                        "measurement receipt parent cannot be inspected after creation",
                        &error,
                    )
                })?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(CustodyError::storage(
                    "measurement receipt parent is absent",
                ))
            }
            Err(error) => {
                return Err(CustodyError::storage(format!(
                    "measurement receipt parent cannot be inspected safely: {error}"
                )))
            }
        };
        let (opened, edge) = open_directory_edge(
            &directory,
            component,
            initial_stat,
            "measurement receipt parent",
            "opened",
        )?;
        edges.push(edge);
        directory = opened;
    }
    Ok((directory, name.to_owned(), edges))
}
