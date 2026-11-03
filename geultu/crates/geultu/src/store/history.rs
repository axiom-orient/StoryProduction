//! Journal-backed history projection for adopted authoring documents.

use super::journal::{ensure_created_project, journal_entries};
use super::lock::acquire_lock;
use super::{validate_project_id, Workspace};
use crate::authoring::Kind;
use crate::domain::evolve;
use crate::error::{Error, Result};

impl Workspace {
    pub fn authoring_history(
        &self,
        id: &str,
        kind: Kind,
        revision: Option<u64>,
    ) -> Result<serde_json::Value> {
        validate_project_id(id)?;
        let _lock = acquire_lock(&self.project_dir(id).join("lock"))?;
        let current = self.load_project(id)?;
        let mut project = None;
        let mut last_revision = 0;
        let mut versions = std::collections::VecDeque::new();
        let mut selected = None;
        let mut total: usize = 0;
        for entry in journal_entries(&self.events_path(id))? {
            let envelope = entry?;
            if envelope.seq == 1 {
                ensure_created_project(id, &envelope.event)?;
            }
            project = Some(evolve(project, &envelope.event)?);
            if let Some(doc) = project
                .as_ref()
                .and_then(|p| p.authoring.documents.get(&kind))
            {
                if doc.revision != last_revision {
                    total += 1;
                    last_revision = doc.revision;
                    versions.push_back(serde_json::json!({"revision":doc.revision,"content_hash":doc.content_hash(),
                        "actor":doc.last_actor,"at_ms":envelope.timestamp_ms,"project_revision":envelope.seq}));
                    if versions.len() > 20 {
                        versions.pop_front();
                    }
                    if revision == Some(doc.revision) {
                        selected = Some(doc.clone());
                    }
                }
            }
        }
        if revision.is_some() && selected.is_none() {
            return Err(Error::NotFound("document revision not found".into()));
        }
        Ok(
            serde_json::json!({"kind":kind,"current_revision":current.authoring.documents.get(&kind).map(|d|d.revision),
            "total_versions":total,"recent_versions":versions,"omitted_versions":total.saturating_sub(20),"document":selected}),
        )
    }
}
