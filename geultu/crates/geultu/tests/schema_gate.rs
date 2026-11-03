//! M4 schema gate (H8): offline Draft 2020-12 validation of the canonical
//! contract package.
//!
//! Everything loads from disk relative to `CARGO_MANIFEST_DIR`
//! (`../../docs/canonical/contracts`, the same ancestor-walking convention
//! as the craft-pack fixtures). All 18 schemas are registered in a
//! pre-built [`jsonschema::Registry`] keyed by their `$id` plus every
//! relative-`$ref` alias (the corpus references siblings as bare filenames,
//! e.g. `"artifact-ref.schema.json"`, which resolve against each referrer's
//! `$id`), so validator construction never needs a retriever — with
//! `default-features = false` the default retriever always fails, making
//! any unresolved reference a loud construction error instead of a network
//! fetch.
//!
//! Fixture verdicts mirror `docs/canonical/tools/verify.py` semantics; if a
//! Rust verdict ever disagrees with the Python authority, this gate fails
//! and the discrepancy is reported instead of papered over.

use jsonschema::Draft;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_SCHEMA_COUNT: usize = 18;

fn contracts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/canonical/contracts")
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/canonical/fixtures")
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_str(&raw).map_err(|error| format!("parse {}: {error}", path.display()))
}

/// Loads every `*.schema.json` under `dir`, keyed by file name, failing
/// loudly when the directory or any document is missing, unparsable, or
/// lacks an `$id`.
fn load_schemas(dir: &Path) -> Result<BTreeMap<String, (String, Value)>, String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|error| format!("contracts directory {}: {error}", dir.display()))?
        .map(|entry| entry.map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    let mut schemas = BTreeMap::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".schema.json") {
            continue;
        }
        let contents = read_json(&entry.path())?;
        let id = contents
            .get("$id")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{} is missing its $id", entry.path().display()))?
            .to_owned();
        schemas.insert(name, (id, contents));
    }
    if schemas.len() != EXPECTED_SCHEMA_COUNT {
        return Err(format!(
            "expected {EXPECTED_SCHEMA_COUNT} contract schemas under {}, found {}",
            dir.display(),
            schemas.len()
        ));
    }
    Ok(schemas)
}

/// Collects every `$ref` string anywhere in a schema document.
fn collect_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key == "$ref" {
                    if let Some(reference) = child.as_str() {
                        refs.push(reference.to_owned());
                    }
                } else {
                    collect_refs(child, refs);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_refs(item, refs);
            }
        }
        _ => {}
    }
}

/// Resolves a bare-filename reference against a referrer's `$id`
/// (`https://host/schema/<name>/v1` + `<file>`); the corpus contains no
/// other reference shape, and anything else fails loudly here.
fn resolve_alias(base_id: &str, reference: &str) -> Result<String, String> {
    if reference.starts_with('#') || reference.contains("://") {
        return Err(format!(
            "unsupported $ref shape '{reference}' in schema '{base_id}'"
        ));
    }
    let cut = base_id
        .rfind('/')
        .ok_or_else(|| format!("$id '{base_id}' has no path segment to resolve against"))?;
    Ok(format!("{}/{}", &base_id[..cut], reference))
}

type RegistryBundle = (
    jsonschema::Registry<'static>,
    BTreeMap<String, (String, Value)>,
);

/// Builds the offline registry: every schema under its `$id`, plus one alias
/// per (referrer, relative $ref) pair pointing at the referenced sibling.
fn build_registry(dir: &Path) -> Result<RegistryBundle, String> {
    let schemas = load_schemas(dir)?;
    let mut pairs: Vec<(String, Value)> = Vec::new();
    for (id, contents) in schemas.values() {
        pairs.push((id.clone(), contents.clone()));
    }
    for (referrer_name, (referrer_id, contents)) in &schemas {
        let mut refs = Vec::new();
        collect_refs(contents, &mut refs);
        for reference in refs {
            let alias = resolve_alias(referrer_id, &reference)?;
            let target = schemas.get(reference.as_str()).ok_or_else(|| {
                format!("{referrer_name} references '{reference}' but no such schema file exists")
            })?;
            pairs.push((alias, target.1.clone()));
        }
    }
    let registry = jsonschema::Registry::new()
        .draft(Draft::Draft202012)
        .extend(pairs)
        .map_err(|error| format!("registry resources rejected: {error}"))?
        .prepare()
        .map_err(|error| format!("registry preparation failed: {error}"))?;
    Ok((registry, schemas))
}

fn validator_for(
    registry: &jsonschema::Registry<'static>,
    schema: &Value,
) -> Result<jsonschema::Validator, String> {
    jsonschema::options()
        .with_registry(registry)
        .build(schema)
        .map_err(|error| format!("validator construction failed: {error}"))
}

#[test]
fn all_contract_schemas_meta_validate_against_draft_2020_12() {
    let schemas = load_schemas(&contracts_dir()).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(schemas.len(), EXPECTED_SCHEMA_COUNT);
    for (name, (id, contents)) in &schemas {
        jsonschema::draft202012::meta::validate(contents)
            .unwrap_or_else(|error| panic!("meta-validation of {name} ({id}) failed: {error}"));
    }
}

#[test]
fn every_schema_builds_a_validator_from_the_prebuilt_offline_registry() {
    let (registry, schemas) =
        build_registry(&contracts_dir()).unwrap_or_else(|error| panic!("{error}"));
    for (name, (id, contents)) in &schemas {
        validator_for(&registry, contents)
            .unwrap_or_else(|error| panic!("validator for {name} ({id}): {error}"));
    }
}

#[test]
fn registry_construction_fails_loudly_when_contracts_are_missing() {
    let missing = std::env::temp_dir().join(format!(
        "geultu-schema-gate-missing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let error = build_registry(&missing).expect_err("a missing contracts dir must fail loudly");
    assert!(
        error.contains("contracts directory"),
        "unexpected error: {error}"
    );

    // A directory that exists but lacks the corpus fails on the count gate.
    fs::create_dir_all(&missing).unwrap();
    let error = build_registry(&missing).expect_err("an empty contracts dir must fail loudly");
    assert!(
        error.contains("expected 18 contract schemas"),
        "unexpected error: {error}"
    );
    fs::remove_dir_all(&missing).unwrap();

    // A single unreadable document fails the whole load, never partially.
    let broken = std::env::temp_dir().join(format!(
        "geultu-schema-gate-broken-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("geultu-story-package.schema.json"), "{not json").unwrap();
    let error = build_registry(&broken).expect_err("a malformed schema must fail loudly");
    assert!(error.contains("parse"), "unexpected error: {error}");
    fs::remove_dir_all(&broken).unwrap();
}

#[test]
fn t33_fixture_verdicts_match_the_python_authority() {
    let (registry, schemas) =
        build_registry(&contracts_dir()).unwrap_or_else(|error| panic!("{error}"));
    let (_, t33) = schemas
        .get("geultu-story-package.schema.json")
        .expect("t33 schema is part of the corpus");
    let validator = validator_for(&registry, t33).unwrap();

    let verdict = |fixture: &str| {
        let instance =
            read_json(&fixtures_dir().join(fixture)).unwrap_or_else(|error| panic!("{error}"));
        validator.is_valid(&instance)
    };

    assert!(
        verdict("t33/valid-minimal.json"),
        "valid-minimal must be accepted, matching verify.py fixture_flat semantics"
    );
    for fixture in [
        "t33/invalid-caller-pass.json",
        "t33/invalid-digest-format.json",
        "t33/invalid-score-out-of-range.json",
        "t33/invalid-legacy-envelope.json",
    ] {
        assert!(
            !verdict(fixture),
            "{fixture} must be rejected, matching verify.py fixture semantics"
        );
    }

    // Each rejection names the constraint category its fixture name implies.
    let first_error = |fixture: &str| {
        let instance =
            read_json(&fixtures_dir().join(fixture)).unwrap_or_else(|error| panic!("{error}"));
        let message = validator
            .iter_errors(&instance)
            .next()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no errors".to_owned());
        message
    };
    let caller_pass = first_error("t33/invalid-caller-pass.json");
    assert!(
        caller_pass.contains("additionalProperties") || caller_pass.contains("passed"),
        "caller-pass must fail the closed-object rule, got: {caller_pass}"
    );
    let digest_format = first_error("t33/invalid-digest-format.json");
    assert!(
        digest_format.to_lowercase().contains("pattern") || digest_format.contains("sha256:"),
        "digest-format must fail the digest pattern, got: {digest_format}"
    );
    let score = first_error("t33/invalid-score-out-of-range.json");
    assert!(
        score.contains("maximum") || score.contains("1"),
        "score-out-of-range must fail the visual_priority maximum, got: {score}"
    );
    let legacy = first_error("t33/invalid-legacy-envelope.json");
    assert!(
        legacy.contains("const") || legacy.contains("required"),
        "legacy envelope must fail the flat-shape identity rules, got: {legacy}"
    );
}

#[test]
fn commit_multi_output_fixture_validates_against_artifact_commit() {
    let (registry, schemas) =
        build_registry(&contracts_dir()).unwrap_or_else(|error| panic!("{error}"));
    let (_, commit) = schemas
        .get("artifact-commit.schema.json")
        .expect("artifact-commit schema is part of the corpus");
    let validator = validator_for(&registry, commit).unwrap();
    let instance = read_json(&fixtures_dir().join("commit/valid-multi-output.json"))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        validator.is_valid(&instance),
        "valid-multi-output.json must validate against artifact-commit.schema.json"
    );
}
