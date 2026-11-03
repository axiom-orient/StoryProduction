//! Request-local freshness against one borrowed, immutable project snapshot.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::Project;
use crate::{Error, Result};

use super::{digest, Block, Document, Kind, Task};

fn block_index(document: Option<&Document>) -> BTreeMap<&str, &Block> {
    let mut index = BTreeMap::new();
    for block in document.into_iter().flat_map(|document| &document.blocks) {
        // This retains the old first-match behavior for direct callers, while
        // persisted document admission separately requires unique block ids.
        index.entry(block.id.as_str()).or_insert(block);
    }
    index
}

pub(super) struct Freshness<'a> {
    project: &'a Project,
    guidance: Option<String>,
    hashes: BTreeMap<Kind, String>,
    current: BTreeMap<Kind, bool>,
    visiting: BTreeSet<Kind>,
}

impl<'a> Freshness<'a> {
    pub(super) fn new(project: &'a Project) -> Self {
        Self {
            project,
            guidance: None,
            hashes: BTreeMap::new(),
            current: BTreeMap::new(),
            visiting: BTreeSet::new(),
        }
    }

    pub(super) fn guidance_hash(&mut self) -> String {
        let project = self.project;
        self.guidance
            .get_or_insert_with(|| digest(&(&project.authoring.brief, &project.voice_profile)))
            .clone()
    }

    pub(super) fn document_hash(&mut self, kind: Kind) -> Option<String> {
        let project = self.project;
        let document = project.authoring.documents.get(&kind)?;
        Some(
            self.hashes
                .entry(kind)
                .or_insert_with(|| document.content_hash())
                .clone(),
        )
    }

    fn is_current(&mut self, kind: Kind) -> bool {
        if let Some(current) = self.current.get(&kind) {
            return *current;
        }
        // Persisted graphs are admitted before use. A direct caller supplying a
        // cycle must still receive a blocker rather than an unbounded recursion.
        if !self.visiting.insert(kind) {
            return false;
        }
        let project = self.project;
        let current = project
            .authoring
            .documents
            .get(&kind)
            .is_some_and(|document| self.document_reasons(document).is_empty());
        self.visiting.remove(&kind);
        self.current.insert(kind, current);
        current
    }

    pub(super) fn document_reasons(&mut self, document: &Document) -> Vec<String> {
        let mut reasons = Vec::new();
        for (kind, expected) in &document.dependencies {
            if self.document_hash(*kind).as_deref() != Some(expected.as_str())
                || !self.is_current(*kind)
            {
                reasons.push(format!(
                    "{} 변경: {}를 다시 작성하거나 직접 반영하세요",
                    kind.label(),
                    document.kind.label()
                ));
            }
        }
        if document.kind != Kind::Source && document.guidance_hash != self.guidance_hash() {
            reasons.push("제작 브리프 또는 글투가 바뀌었습니다".into());
        }
        reasons
    }

    pub(super) fn task_reasons(&mut self, task: &Task) -> Vec<String> {
        let mut reasons = Vec::new();
        if task.cancelled {
            reasons.push("작업이 중단되었습니다".into());
        }
        if task.guidance_hash != self.guidance_hash() {
            reasons.push("브리프 또는 글투가 변경되었습니다".into());
        }
        let project = self.project;
        for (kind, input) in &task.inputs {
            let content_matches = project
                .authoring
                .documents
                .get(kind)
                .is_some_and(|current| current.blocks == input.blocks);
            if !content_matches || !self.is_current(*kind) {
                reasons.push(format!("{} 기준이 변경되었습니다", kind.label()));
            }
        }
        let current = project.authoring.documents.get(&task.target);
        if task.selection.is_empty() {
            // Document content identity covers blocks only. Compare those exact
            // values without serializing and hashing both full snapshots again.
            if current.map(|document| document.blocks.as_slice())
                != task
                    .base
                    .as_ref()
                    .map(|document| document.blocks.as_slice())
            {
                reasons
                    .push("대상 문서가 변경되었습니다. 최신 문서에서 새 작업을 준비하세요".into());
            }
        } else {
            let original = block_index(task.base.as_ref());
            let latest = block_index(current);
            for id in &task.selection {
                if original.get(id.as_str()) != latest.get(id.as_str()) {
                    reasons.push(format!("선택 블록 {id}가 변경되었습니다"));
                }
            }
        }
        reasons
    }

    pub(super) fn dependencies(&mut self, kind: Kind) -> Result<BTreeMap<Kind, String>> {
        let mut result = BTreeMap::new();
        for upstream in kind.upstream() {
            let hash = self.document_hash(*upstream).ok_or_else(|| {
                Error::Conflict(format!("{}를 먼저 작성하세요", upstream.label()))
            })?;
            if !self.is_current(*upstream) {
                return Err(Error::Conflict(format!(
                    "{}가 현재 원문/브리프와 다릅니다",
                    upstream.label()
                )));
            }
            result.insert(*upstream, hash);
        }
        Ok(result)
    }
}

pub fn stale_reasons(project: &Project, document: &Document) -> Vec<String> {
    Freshness::new(project).document_reasons(document)
}
