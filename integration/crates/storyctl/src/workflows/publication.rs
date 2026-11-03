//! Integrity-bound transport publication. The owners decide release eligibility.
use super::text::{string, TextContext};
use crate::{
    adapters::owners::path_string,
    effects::{files, hash, json},
    error::{Error, Result},
};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn writing_files(writing: &Value) -> Result<Map<String, Value>> {
    let documents = writing["documents"]
        .as_object()
        .ok_or_else(|| Error::not_applied("writing bundle documents invalid"))?;
    let expected = BTreeSet::from(["source", "story", "synopsis"]);
    if documents
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected
        || writing["project_revision"].as_u64().is_none()
    {
        return Err(Error::not_applied(
            "writing bundle must contain three documents at one revision",
        ));
    }
    let mut result = Map::new();
    for (kind, doc) in documents {
        if doc["kind"] != *kind
            || doc["project_revision"] != writing["project_revision"]
            || doc["revision"]
                .as_u64()
                .is_none_or(|revision| revision == 0)
        {
            return Err(Error::not_applied(
                "writing document does not belong to captured bundle",
            ));
        }
        result.insert(format!("{kind}.md"),json!({"text":string(doc,"markdown")?,"sha256":format!("sha256:{}",string(doc,"markdown_sha256")?)}));
    }
    Ok(result)
}

/// Validate the existing producer DTOs and their declared identity, rather than
/// deciding approval or reconstructing the producer's canonical content hashes.
fn confirmed_release(
    ctx: &TextContext,
    release: &Value,
    artifacts: &Map<String, Value>,
) -> Result<()> {
    let decode = |name: &str| -> Result<Value> {
        json::parse_object(string(&artifacts[name], "text")?.as_bytes())
            .map_err(|_| Error::unknown(format!("NVC release contains invalid JSON: {name}")))
    };
    let board = decode("storyboard.json")?;
    let review = decode("storyboard-review.json")?;
    decode("scene-map.json")?;
    let schemas =
        crate::contracts::SchemaRegistry::new(&ctx.repository.join("integration/contracts"))?;
    for (schema, value) in [
        (
            "https://narrative-courier.local/contracts/text-storyboard-v1.schema.json",
            &board,
        ),
        (
            "https://narrative-courier.local/contracts/text-storyboard-review-v1.schema.json",
            &review,
        ),
    ] {
        schemas
            .validate(schema, value)
            .map_err(|error| Error::unknown(format!("NVC release DTO is invalid: {error}")))?;
    }
    if board["contentHash"] != release["boardHash"]
        || review["boardHash"] != release["boardHash"]
        || board["source"] != release["source"]
        || review["source"] != release["source"]
        || review["decision"] != "approved"
    {
        return Err(Error::unknown(
            "NVC release, board and approved review acknowledgment disagree",
        ));
    }
    Ok(())
}
pub(crate) fn validate_narrative_release(ctx: &TextContext, release: &Value) -> Result<()> {
    let narrative = release["files"]
        .as_object()
        .ok_or_else(|| Error::unknown("NVC release files are missing"))?;
    let expected = BTreeSet::from([
        "screenplay.md",
        "scene-blueprint.md",
        "scene-map.json",
        "storyboard.md",
        "storyboard.json",
        "storyboard-review.json",
    ]);
    if release["status"] != "approved_text_package"
        || !release["boardHash"].as_str().is_some_and(hash::is_sha256)
        || !release["synopsisSha256"]
            .as_str()
            .is_some_and(|value| hash::is_sha256(&format!("sha256:{value}")))
        || narrative
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != expected
    {
        return Err(Error::unknown(
            "NVC release lacks its exact approved artifact acknowledgment",
        ));
    }
    capture(narrative).map_err(|error| {
        Error::unknown(format!(
            "NVC release artifact acknowledgment is invalid: {error}"
        ))
    })?;
    confirmed_release(ctx, release, narrative)
}

fn capture(files: &Map<String, Value>) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut result = BTreeMap::new();
    for (name, artifact) in files {
        let bytes = string(artifact, "text")?.as_bytes().to_vec();
        if artifact["sha256"] != hash::sha256_bytes(&bytes) {
            return Err(Error::not_applied(format!(
                "producer bytes differ from hash: {name}"
            )));
        }
        result.insert(name.clone(), bytes);
    }
    Ok(result)
}
pub fn export(ctx: &TextContext, output: &Path) -> Result<Value> {
    let writing = ctx.author("authoring.bundle", json!({}))?;
    let project = ctx.project_id()?;
    let captured = capture(&writing_files(&writing)?)?;
    let edition = files::PublicationDirectory::new(output)?;
    for (name, bytes) in &captured {
        edition.immutable(name, bytes)?;
    }
    let mut docs = Map::new();
    for kind in ["source", "story", "synopsis"] {
        let doc = &writing["documents"][kind];
        docs.insert(kind.into(),json!({"path":format!("{kind}.md"),"sha256":doc["markdown_sha256"],"revision":doc["revision"]}));
    }
    let mut manifest = json!({"schemaVersion":"story.text-export/1","projectId":project,"writingRevision":writing["project_revision"],"status":"writing_documents_exported","documents":docs,"nextAction":"finish STS and use storyboard export for scenes and the reviewed text board"});
    if ctx.project_id()? != project || ctx.author("authoring.bundle", json!({}))? != writing {
        return Err(Error::not_applied(
            "writing changed during export; this edition was not committed",
        ));
    }
    edition.verify(&captured)?;
    if crate::effects::process::cancellation_requested() {
        return Err(Error::unknown(
            "export interrupted before completion marker; inspect partial edition",
        ));
    }
    edition
        .immutable("writing-manifest.json", &json::stable_bytes(&manifest)?)
        .map_err(|error| {
            Error::unknown(format!(
                "writing manifest publication not confirmed; inspect {}: {error}",
                output.display()
            ))
        })?;
    manifest["directory"] = json!(path_string(&files::non_symlink_path(output)?)?);
    Ok(manifest)
}
pub fn package(ctx: &TextContext, board: &str, review: &str, output: &Path) -> Result<Value> {
    if !ctx.source_current()? {
        return Err(Error::not_applied(
            "current writing synopsis differs from released STS source",
        ));
    }
    let writing = ctx.author("authoring.bundle", json!({}))?;
    let project = ctx.project_id()?;
    let mut artifacts = writing_files(&writing)?;
    let options = vec![
        "bundle".into(),
        "--board".into(),
        board.into(),
        "--review".into(),
        review.into(),
    ];
    let release = ctx.board(&options)?;
    validate_narrative_release(ctx, &release)?;
    let narrative = release["files"]
        .as_object()
        .expect("release artifact set admitted");
    if artifacts["synopsis.md"]["sha256"]
        != format!("sha256:{}", string(&release, "synopsisSha256")?)
    {
        return Err(Error::not_applied(
            "writing snapshot does not match released synopsis",
        ));
    }
    artifacts.extend(narrative.clone());
    let captured = capture(&artifacts)?;
    let edition = files::PublicationDirectory::new(output)?;
    for (name, bytes) in &captured {
        edition.immutable(name, bytes)?;
    }
    let hashes: Map<String, Value> = artifacts
        .iter()
        .map(|(name, a)| (name.clone(), json!({"sha256":a["sha256"]})))
        .collect();
    let manifest = json!({"schemaVersion":"story.text-package/1","status":"complete","projectId":project,"writingRevision":writing["project_revision"],"source":release["source"],"boardHash":release["boardHash"],"files":hashes});
    if ctx.project_id()? != project || ctx.author("authoring.bundle", json!({}))? != writing {
        return Err(Error::not_applied(
            "writing changed during package publication; edition not committed",
        ));
    }
    if ctx.board(&options)? != release {
        return Err(Error::not_applied(
            "NVC release or storyboard approval changed during publication; edition not committed",
        ));
    }
    edition.verify(&captured)?;
    if crate::effects::process::cancellation_requested() {
        return Err(Error::unknown(
            "package interrupted before completion marker; inspect partial edition",
        ));
    }
    edition
        .immutable("story-package.json", &json::stable_bytes(&manifest)?)
        .map_err(|error| {
            Error::unknown(format!(
                "package marker publication not confirmed; inspect {}: {error}",
                output.display()
            ))
        })?;
    Ok(
        json!({"status":"complete","manifest":path_string(&files::non_symlink_path(&output.join("story-package.json"))?)?,"files":artifacts.keys().collect::<Vec<_>>()}),
    )
}
