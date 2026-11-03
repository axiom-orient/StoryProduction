mod file_io;
mod handoff;
mod history;
mod journal;
mod lock;
mod project;
mod recovery;
mod transaction;

#[cfg(test)]
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::model::{Project, WorkspaceMeta, SCHEMA_VERSION};
use file_io::{absolutize, atomic_json, read_json};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const STATE_DIR: &str = ".geultu";

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot<T = Project> {
    event_count: u64,
    project: T,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitResult {
    pub project_id: String,
    pub revision: u64,
    pub event_count: usize,
    pub idempotent: bool,
}

impl Workspace {
    pub fn init(root: impl AsRef<Path>) -> Result<Self> {
        let workspace = Self {
            root: absolutize(root.as_ref())?,
        };
        fs::create_dir_all(workspace.root.join(STATE_DIR).join("projects"))?;
        let meta_path = workspace.meta_path();
        if !meta_path.exists() {
            let meta = WorkspaceMeta {
                schema_version: SCHEMA_VERSION,
                engine_version: ENGINE_VERSION.into(),
                created_at_ms: now_ms(),
                projects: Default::default(),
            };
            atomic_json(&meta_path, &meta)?;
        } else {
            workspace.metadata()?;
        }
        Ok(workspace)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let workspace = Self {
            root: absolutize(root.as_ref())?,
        };
        if !workspace.meta_path().exists() {
            return Err(Error::NotFound(format!(
                "workspace is not initialized: {} (run `geultu --workspace PATH init`)",
                workspace.root.display()
            )));
        }
        workspace.metadata()?;
        Ok(workspace)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn metadata(&self) -> Result<WorkspaceMeta> {
        let meta: WorkspaceMeta = read_json(&self.meta_path())?;
        if meta.schema_version != SCHEMA_VERSION {
            return Err(Error::Corrupt(format!(
                "unsupported workspace schema {}",
                meta.schema_version
            )));
        }
        Ok(meta)
    }

    fn meta_path(&self) -> PathBuf {
        self.root.join(STATE_DIR).join("workspace.json")
    }

    fn project_dir(&self, id: &str) -> PathBuf {
        self.root.join(STATE_DIR).join("projects").join(id)
    }

    fn events_path(&self, id: &str) -> PathBuf {
        self.project_dir(id).join("events.ndjson")
    }

    fn snapshot_path(&self, id: &str) -> PathBuf {
        self.project_dir(id).join("snapshot.json")
    }
}

fn validate_project_id(id: &str) -> Result<()> {
    crate::domain::validation::validate_id("project id", id)?;
    if matches!(id, "." | "..") {
        return Err(Error::InvalidInput(
            "project id must name a project directory".into(),
        ));
    }
    Ok(())
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::lock::acquire_lock;
    use super::*;
    use crate::model::Scene;

    fn temp_workspace(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "geultu-{label}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn committed_journal_recovers_without_snapshot() {
        let root = temp_workspace("recovery");
        let ws = Workspace::init(&root).unwrap();
        ws.create_project("book", "Book", "cmd-create").unwrap();
        ws.transact(
            "book",
            "cmd-scene",
            Command::AddScene {
                scene: Scene {
                    id: "s1".into(),
                    title: "One".into(),
                    text: "text".into(),
                    order: 0,
                    revision: 0,
                },
            },
        )
        .unwrap();
        fs::remove_file(ws.snapshot_path("book")).unwrap();
        let recovered = ws.recover_project("book", false).unwrap();
        assert_eq!(recovered.scenes.len(), 1);
        assert_eq!(recovered.revision, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn same_command_is_idempotent() {
        let root = temp_workspace("idempotency");
        let ws = Workspace::init(&root).unwrap();
        ws.create_project("book", "Book", "cmd-create").unwrap();
        let command = Command::AddScene {
            scene: Scene {
                id: "s1".into(),
                title: "One".into(),
                text: String::new(),
                order: 0,
                revision: 0,
            },
        };
        let first = ws.transact("book", "cmd-scene", command.clone()).unwrap();
        let second = ws.transact("book", "cmd-scene", command).unwrap();
        assert!(!first.idempotent);
        assert!(second.idempotent);
        assert_eq!(first.revision, second.revision);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn index_lock_failure_reports_committed_revision() {
        let root = temp_workspace("committed-index");
        let ws = Workspace::init(&root).unwrap();
        ws.create_project("book", "Book", "create").unwrap();
        let lock = acquire_lock(&root.join(STATE_DIR).join("workspace.lock")).unwrap();
        let error = ws
            .transact(
                "book",
                "scene",
                Command::AddScene {
                    scene: Scene {
                        id: "s1".into(),
                        title: "One".into(),
                        text: "text".into(),
                        order: 0,
                        revision: 0,
                    },
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            Error::CommittedButSnapshotFailed { revision: 2, .. }
        ));
        assert_eq!(ws.load_project("book").unwrap().revision, 2);
        drop(lock);
        assert_eq!(ws.recover_index().unwrap().projects["book"].revision, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn future_journal_schema_is_rejected_even_under_snapshot() {
        let root = temp_workspace("future-journal");
        let ws = Workspace::init(&root).unwrap();
        ws.create_project("book", "Book", "create").unwrap();
        let path = ws.events_path("book");
        let mut value: serde_json::Value =
            serde_json::from_str(fs::read_to_string(&path).unwrap().trim()).unwrap();
        value["schema_version"] = serde_json::json!(SCHEMA_VERSION + 1);
        fs::write(&path, format!("{}\n", value)).unwrap();
        assert!(matches!(ws.load_project("book"), Err(Error::Corrupt(_))));
        assert!(matches!(
            ws.recover_project("book", false),
            Err(Error::Corrupt(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reviewed_export_respects_project_lock() {
        let root = temp_workspace("handoff-lock");
        let ws = Workspace::init(&root).unwrap();
        ws.create_project("book", "Book", "create").unwrap();
        let lock = acquire_lock(&ws.project_dir("book").join("lock")).unwrap();
        assert!(matches!(
            ws.export_reviewed_synopsis("book", "reviewer", "ko", None, 1, &"0".repeat(64)),
            Err(Error::Busy(_))
        ));
        assert!(!ws.project_dir("book").join("handoff").exists());
        drop(lock);
        fs::remove_dir_all(root).unwrap();
    }
}
