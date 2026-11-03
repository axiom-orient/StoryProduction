//! Integrity failures must leave an edition without its completion marker.
use serde_json::{json, Map, Value};
use std::{
    cell::Cell,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};
use storyctl::{
    effects::hash,
    error::Result,
    ports::{NarrativeOwner, WritingOwner},
    workflows::{publication, text::TextContext},
};

#[derive(Clone, Copy)]
enum Drift {
    None,
    Bytes,
    Symlink,
    Directory,
}
struct Mutation {
    output: PathBuf,
    kind: Drift,
}
impl Mutation {
    fn apply(&self) {
        match self.kind {
            Drift::None => (),
            Drift::Bytes => {
                fs::write(self.output.join("synopsis.md"), b"changed after staging").unwrap()
            }
            Drift::Symlink => {
                fs::remove_file(self.output.join("synopsis.md")).unwrap();
                symlink("source.md", self.output.join("synopsis.md")).unwrap();
            }
            Drift::Directory => {
                fs::rename(&self.output, self.output.with_extension("original")).unwrap();
                fs::create_dir(&self.output).unwrap();
            }
        }
    }
}
fn writing_bundle() -> Value {
    let mut documents = Map::new();
    for kind in ["source", "story", "synopsis"] {
        let markdown = format!("{kind}\n");
        documents.insert(kind.into(), json!({"kind":kind,"project_revision":10,"revision":2,"markdown":markdown,"markdown_sha256":hash::sha256_hex_bytes(markdown.as_bytes())}));
    }
    json!({"project_revision":10,"documents":documents})
}
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}
fn narrative_release() -> Value {
    let board: Value = serde_json::from_str(include_str!(
        "../../../../NarrativeCourier/crates/authoring/tests/fixtures/legacy-board.json"
    ))
    .unwrap();
    let review: Value = serde_json::from_str(include_str!(
        "../../../../NarrativeCourier/crates/authoring/tests/fixtures/legacy-review.json"
    ))
    .unwrap();
    let mut artifacts = Map::new();
    for name in [
        "screenplay.md",
        "scene-blueprint.md",
        "scene-map.json",
        "storyboard.md",
        "storyboard.json",
        "storyboard-review.json",
    ] {
        let text = match name {
            "storyboard.json" => board.to_string(),
            "storyboard-review.json" => review.to_string(),
            "scene-map.json" => "{}".into(),
            _ => format!("fixture {name}\n"),
        };
        artifacts.insert(
            name.into(),
            json!({"sha256":hash::sha256_bytes(text.as_bytes()),"text":text}),
        );
    }
    json!({"status":"approved_text_package","source":board["source"],"boardHash":board["contentHash"],"synopsisSha256":hash::sha256_hex_bytes(b"synopsis\n"),"files":artifacts})
}
struct Writing {
    calls: Cell<usize>,
    mutation: Mutation,
}
impl WritingOwner for Writing {
    fn initialize(&self) -> Result<()> {
        panic!("publication must not initialize owner state")
    }
    fn call(&self, operation: &str, _: &Value) -> Result<Value> {
        match operation {
            "authoring.export" => {
                Ok(json!({"markdown_sha256":hash::sha256_hex_bytes(b"synopsis\n")}))
            }
            "authoring.bundle" => {
                let count = self.calls.get() + 1;
                self.calls.set(count);
                if count == 2 {
                    self.mutation.apply();
                }
                Ok(writing_bundle())
            }
            _ => panic!("unexpected writing operation {operation}"),
        }
    }
}
struct Narrative {
    calls: Cell<usize>,
    mutation: Mutation,
}
impl NarrativeOwner for Narrative {
    fn text(&self, command: &str, _: &str, _: &[String]) -> Result<String> {
        assert_eq!(command, "source-binding");
        Ok(json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":hash::sha256_hex_bytes(b"synopsis\n")}}).to_string())
    }
    fn tool(&self, lane: &str, arguments: &[String], _: &Path) -> Result<Value> {
        assert_eq!(lane, "storyboard");
        assert_eq!(arguments[0], "bundle");
        let count = self.calls.get() + 1;
        self.calls.set(count);
        if count == 2 {
            self.mutation.apply();
        }
        Ok(narrative_release())
    }
}
fn context(
    root: &Path,
    output: &Path,
    writing_drift: Drift,
    narrative_drift: Drift,
) -> TextContext {
    fs::write(
        root.join("story-project.json"),
        br#"{"schemaVersion":"story.text-project/1","projectId":"test"}"#,
    )
    .unwrap();
    TextContext::with_owners(
        repository(),
        root.into(),
        Box::new(Writing {
            calls: Cell::new(0),
            mutation: Mutation {
                output: output.into(),
                kind: writing_drift,
            },
        }),
        Box::new(Narrative {
            calls: Cell::new(0),
            mutation: Mutation {
                output: output.into(),
                kind: narrative_drift,
            },
        }),
        None,
    )
    .unwrap()
}

struct InvalidRelease(Value);
impl NarrativeOwner for InvalidRelease {
    fn text(&self, command: &str, _: &str, _: &[String]) -> Result<String> {
        assert_eq!(command, "source-binding");
        Ok(json!({"reviewed_synopsis":{"source_project_id":"test","synopsis_sha256":hash::sha256_hex_bytes(b"synopsis\n")}}).to_string())
    }
    fn tool(&self, _: &str, _: &[String], _: &Path) -> Result<Value> {
        Ok(self.0.clone())
    }
}

struct InvalidWriting(Value);
impl WritingOwner for InvalidWriting {
    fn initialize(&self) -> Result<()> {
        unreachable!()
    }
    fn call(&self, operation: &str, _: &Value) -> Result<Value> {
        assert_eq!(operation, "authoring.bundle");
        Ok(self.0.clone())
    }
}

#[test]
fn writing_export_cannot_publish_without_each_document_revision() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let output = root.join("edition");
    let mut ctx = context(&root, &output, Drift::None, Drift::None);
    let mut writing = writing_bundle();
    writing["documents"]["story"]
        .as_object_mut()
        .unwrap()
        .remove("revision");
    ctx.writing = Box::new(InvalidWriting(writing));
    assert_eq!(
        publication::export(&ctx, &output)
            .unwrap_err()
            .outcome
            .as_str(),
        "not_applied"
    );
    assert!(!output.exists());
}

#[test]
fn status_approval_observation_rejects_an_incomplete_bundle_acknowledgment() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut ctx = context(&root, &root.join("edition"), Drift::None, Drift::None);
    let project = root.join("narrative/test/storyboards");
    fs::create_dir_all(project.join("boards")).unwrap();
    fs::create_dir_all(project.join("reviews")).unwrap();
    fs::write(
        project.join("boards/board.json"),
        include_str!(
            "../../../../NarrativeCourier/crates/authoring/tests/fixtures/legacy-board.json"
        ),
    )
    .unwrap();
    fs::write(
        project.join("reviews/review.json"),
        include_str!(
            "../../../../NarrativeCourier/crates/authoring/tests/fixtures/legacy-review.json"
        ),
    )
    .unwrap();
    ctx.narrative = Box::new(InvalidRelease(json!({"status":"approved_text_package"})));
    assert_eq!(
        storyctl::workflows::text::approved_board(&ctx)
            .unwrap_err()
            .outcome
            .as_str(),
        "outcome_unknown"
    );
}

#[test]
fn package_never_trusts_a_rejected_or_mismatched_review_inside_an_approved_envelope() {
    for change in ["rejected", "wrong-board", "wrong-source", "invalid-json"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let output = root.join("edition");
        let mut ctx = context(&root, &output, Drift::None, Drift::None);
        let mut release = narrative_release();
        let mut review: Value = serde_json::from_str(
            release["files"]["storyboard-review.json"]["text"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        match change {
            "rejected" => review["decision"] = json!("rejected"),
            "wrong-board" => review["boardHash"] = json!(format!("sha256:{}", "a".repeat(64))),
            "wrong-source" => {
                review["source"]["sceneMapSha256"] = json!(format!("sha256:{}", "a".repeat(64)))
            }
            "invalid-json" => (),
            _ => unreachable!(),
        }
        let text = if change == "invalid-json" {
            "[]".into()
        } else {
            review.to_string()
        };
        release["files"]["storyboard-review.json"] =
            json!({"sha256":hash::sha256_bytes(text.as_bytes()),"text":text});
        ctx.narrative = Box::new(InvalidRelease(release));
        assert_eq!(
            publication::package(&ctx, "board", "review", &output)
                .unwrap_err()
                .outcome
                .as_str(),
            "outcome_unknown"
        );
        assert!(
            !output.exists(),
            "invalid acknowledgment must be rejected before staging"
        );
    }
}
#[test]
fn export_rechecks_staged_bytes_and_directory_before_manifest() {
    for drift in [Drift::Bytes, Drift::Symlink, Drift::Directory] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let output = root.join("edition");
        let ctx = context(&root, &output, drift, Drift::None);
        let error = publication::export(&ctx, &output).unwrap_err();
        assert_eq!(error.outcome.as_str(), "not_applied");
        assert!(!output.join("writing-manifest.json").exists());
        assert!(!output
            .with_extension("original")
            .join("writing-manifest.json")
            .exists());
    }
}
#[test]
fn package_rechecks_disk_after_final_owner_observation() {
    for drift in [Drift::Bytes, Drift::Directory] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let output = root.join("edition");
        let ctx = context(&root, &output, Drift::None, drift);
        let error = publication::package(&ctx, "board.json", "review.json", &output).unwrap_err();
        assert_eq!(error.outcome.as_str(), "not_applied");
        assert!(!output.join("story-package.json").exists());
        assert!(!output
            .with_extension("original")
            .join("story-package.json")
            .exists());
    }
}
#[test]
fn unchanged_edition_replays_equal_bytes_and_exact_nine_file_package() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let output = root.join("edition");
    let ctx = context(&root, &output, Drift::None, Drift::None);
    assert_eq!(
        publication::export(&ctx, &output).unwrap()["status"],
        "writing_documents_exported"
    );
    assert_eq!(
        publication::export(&ctx, &output).unwrap()["status"],
        "writing_documents_exported"
    );
    let result = publication::package(&ctx, "board.json", "review.json", &output).unwrap();
    assert_eq!(result["status"], "complete");
    assert_eq!(result["files"].as_array().unwrap().len(), 9);
    assert!(output.join("story-package.json").is_file());
}
