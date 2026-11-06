//! SAC's animatic PNG pack owner. Story invokes this public CLI and pinned contracts.
mod contracts;
mod files;
mod gif;
mod png;
use chrono::{SecondsFormat, Utc};
use contracts::{canonical, parse, seal, sha256, Contracts};
use files::Root;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub unknown: bool,
}
impl Error {
    pub fn refused(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            unknown: false,
        }
    }
    pub fn unknown(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            unknown: true,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::refused(value.to_string())
    }
}
pub(crate) fn check(condition: bool, code: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::refused(code))
    }
}

pub struct Options {
    pub operation: String,
    pub project_root: PathBuf,
    pub owner_root: PathBuf,
    pub packet: String,
    pub draft: Option<String>,
    pub pack: Option<String>,
    pub output: Option<String>,
    pub expected_hash: Option<String>,
    pub reviewer: Option<String>,
    pub decision: Option<String>,
    pub note: String,
}

fn required(value: &Option<String>) -> Result<&str> {
    value
        .as_deref()
        .ok_or_else(|| Error::refused("MISSING_COMMAND_ARGUMENT"))
}
fn read_json(root: &Root, relative: &str) -> Result<Value> {
    parse(&root.read(relative, 16 * 1024 * 1024)?)
}

fn packet(root: &Root, relative: &str, contracts: &Contracts) -> Result<Value> {
    let value = read_json(root, relative)?;
    contracts.validate("nvc", "animatic-packet-v1", &value)?;
    contracts.validate("nvc", "animatic-plan-v1", &value["plan"])?;
    contracts.validate("nvc", "animatic-review-v1", &value["review"])?;
    check(
        value["review"]["decision"] == "approved"
            && value["review"]["planHash"] == value["plan"]["contentHash"]
            && value["review"]["source"] == value["plan"]["source"],
        "DIRECTION_NOT_APPROVED",
    )?;
    Ok(value)
}

fn validate_pack(root: &Root, value: &Value, source: &Value, contracts: &Contracts) -> Result<()> {
    contracts.validate("sac", "animatic-assets-v1", value)?;
    check(
        value["packetHash"] == source["contentHash"],
        "PACK_SOURCE_CHANGED",
    )?;
    let requirements = source["plan"]["assets"]
        .as_array()
        .ok_or_else(|| Error::refused("ASSET_REQUIREMENTS_MISSING"))?
        .iter()
        .map(|a| (a["assetId"].as_str().unwrap_or(""), a))
        .collect::<BTreeMap<_, _>>();
    let assets = value["assets"]
        .as_array()
        .ok_or_else(|| Error::refused("ASSET_COVERAGE"))?;
    let mut ids = BTreeSet::new();
    for asset in assets {
        let id = asset["assetId"]
            .as_str()
            .ok_or_else(|| Error::refused("ASSET_COVERAGE"))?;
        check(ids.insert(id), "ASSET_COVERAGE")?;
        let required = requirements
            .get(id)
            .ok_or_else(|| Error::refused("ASSET_COVERAGE"))?;
        let path = asset["path"]
            .as_str()
            .ok_or_else(|| Error::refused("PNG_PATH_REQUIRED"))?;
        let raw = root.read(path, 64 * 1024 * 1024)?;
        check(
            path.to_ascii_lowercase().ends_with(".png")
                && asset["sha256"] == format!("sha256:{}", sha256(&raw)),
            "PNG_HASH_CHANGED",
        )?;
        let observed = png::inspect(&raw)?;
        check(
            asset["width"].as_f64() == required["width"].as_f64()
                && asset["height"].as_f64() == required["height"].as_f64()
                && asset["width"].as_f64() == Some(observed.width as f64)
                && asset["height"].as_f64() == Some(observed.height as f64),
            "PNG_DIMENSIONS_CHANGED",
        )?;
        check(
            asset["alpha"] == required["alpha"],
            "ALPHA_REQUIREMENT_CHANGED",
        )?;
        check(
            observed.has_transparent_pixels == (asset["alpha"] == "cutout"),
            "PNG_ALPHA_MISMATCH",
        )?;
        check(
            observed.alpha_max.is_none_or(|max| max > 0),
            "PNG_ENTIRELY_TRANSPARENT",
        )?;
        if asset["provenance"]["executor"] == "scene-provider-v2" {
            check(
                asset["alpha"] == "opaque",
                "CUTOUT_REQUIRES_CODEX_IMAGEGEN_OR_USER_ASSET",
            )?;
        }
    }
    check(
        ids == requirements.keys().copied().collect(),
        "ASSET_COVERAGE",
    )
}

fn publish(root: &Root, relative: &str, value: &Value) -> Result<()> {
    let mut raw = canonical(value)?;
    raw.push(b'\n');
    root.publish(relative, &raw)
}

pub fn run(options: &Options) -> Result<Value> {
    check(
        ["submit", "inspect", "review", "import-gif", "inspect-gif"]
            .contains(&options.operation.as_str()),
        "UNKNOWN_OPERATION",
    )?;
    let root = Root::new(&options.project_root)?;
    let contracts = Contracts::new(&options.owner_root)?;
    if !matches!(options.operation.as_str(), "inspect" | "inspect-gif") {
        check(
            required(&options.output)?.starts_with("animatic/"),
            "OUTPUT_MUST_BE_UNDER_ANIMATIC",
        )?;
    }
    let source = packet(&root, &options.packet, &contracts)?;
    if options.operation == "inspect-gif" {
        return gif::inspect(&root, options, &source, &contracts);
    }
    if options.operation == "import-gif" {
        return gif::import(&root, options, &source, &contracts);
    }
    let mut value = read_json(
        &root,
        if options.operation == "submit" {
            required(&options.draft)?
        } else {
            required(&options.pack)?
        },
    )?;
    if options.operation == "submit" {
        check(
            value.get("contentHash").is_none(),
            "DRAFT_MUST_NOT_BE_SEALED",
        )?;
        value = seal(value)?;
    }
    validate_pack(&root, &value, &source, &contracts)?;
    if options.operation == "submit" {
        publish(&root, required(&options.output)?, &value)?;
        return Ok(
            json!({"status":"candidate","path":options.output,"contentHash":value["contentHash"]}),
        );
    }
    if options.operation == "inspect" {
        return Ok(
            json!({"status":"inspected","packHash":value["contentHash"],"packetHash":source["contentHash"]}),
        );
    }
    check(
        value["contentHash"] == required(&options.expected_hash)?,
        "REVIEW_HASH_CHANGED",
    )?;
    let reviewer = required(&options.reviewer)?;
    check(!reviewer.trim().is_empty(), "REVIEWER_REQUIRED")?;
    let decision = required(&options.decision)?;
    check(
        ["approved", "rejected"].contains(&decision),
        "REVIEW_DECISION_INVALID",
    )?;
    let mut receipt = seal(
        json!({"schemaVersion":"sac.animatic-assets-review/1","packHash":value["contentHash"],
        "packetHash":source["contentHash"],"decision":decision,"reviewer":reviewer,"note":options.note,
        "reviewedAt":Utc::now().to_rfc3339_opts(SecondsFormat::Micros,false)}),
    )?;
    contracts.validate("sac", "animatic-assets-review-v1", &receipt)?;
    let output = required(&options.output)?;
    if root.exists(output)? {
        let existing = read_json(&root, output)?;
        contracts.validate("sac", "animatic-assets-review-v1", &existing)?;
        check(
            receipt.as_object().unwrap().iter().all(|(key, value)| {
                ["contentHash", "reviewedAt"].contains(&key.as_str())
                    || existing.get(key) == Some(value)
            }),
            "IMMUTABLE_REVIEW_CONFLICT",
        )?;
        receipt = existing;
    }
    publish(&root, output, &receipt)?;
    Ok(receipt)
}

pub fn parse_options(arguments: &[String]) -> Result<Options> {
    let operation = arguments
        .first()
        .ok_or_else(|| Error::refused("MISSING_COMMAND_ARGUMENT"))?
        .to_owned();
    let mut values = BTreeMap::new();
    let mut index = 1;
    let allowed = [
        "project-root",
        "owner-root",
        "packet",
        "draft",
        "pack",
        "output",
        "expected-hash",
        "reviewer",
        "decision",
        "note",
    ];
    while index < arguments.len() {
        let key = arguments[index]
            .strip_prefix("--")
            .filter(|k| allowed.contains(k))
            .ok_or_else(|| Error::refused("UNKNOWN_OPTION"))?;
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| Error::refused("MISSING_OPTION_VALUE"))?;
        check(
            values.insert(key.to_owned(), value.clone()).is_none(),
            "DUPLICATE_OPTION",
        )?;
        index += 2;
    }
    let owner_root = values
        .remove("owner-root")
        .or_else(|| std::env::var("SAC_OWNER_ROOT").ok())
        .map(PathBuf::from);
    let owner_root = match owner_root {
        Some(root) => root,
        None => std::env::current_exe()?
            .ancestors()
            .find(|path| {
                path.join("docs/contracts/animatic-assets-v1.schema.json")
                    .is_file()
                    && path
                        .join("contracts/narrative-courier/animatic-packet-v1.schema.json")
                        .is_file()
            })
            .map(PathBuf::from)
            .ok_or_else(|| {
                Error::refused("SAC_RESOURCES_MISSING: pass --owner-root or SAC_OWNER_ROOT")
            })?,
    };
    let project_root = PathBuf::from(
        values
            .remove("project-root")
            .ok_or_else(|| Error::refused("MISSING_COMMAND_ARGUMENT"))?,
    );
    let packet = values
        .remove("packet")
        .ok_or_else(|| Error::refused("MISSING_COMMAND_ARGUMENT"))?;
    let options = Options {
        operation,
        owner_root,
        project_root,
        packet,
        draft: values.remove("draft"),
        pack: values.remove("pack"),
        output: values.remove("output"),
        expected_hash: values.remove("expected-hash"),
        reviewer: values.remove("reviewer"),
        decision: values.remove("decision"),
        note: values.remove("note").unwrap_or_default(),
    };
    match options.operation.as_str() {
        "submit" | "import-gif" => {
            required(&options.draft)?;
            required(&options.output)?;
        }
        "inspect" | "inspect-gif" => {
            required(&options.pack)?;
        }
        "review" => {
            for v in [
                &options.pack,
                &options.output,
                &options.expected_hash,
                &options.reviewer,
                &options.decision,
            ] {
                required(v)?;
            }
        }
        _ => return Err(Error::refused("UNKNOWN_OPERATION")),
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(alpha: u8, alpha_policy: &str) -> (tempfile::TempDir, Options, Value) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("animatic")).unwrap();
        let hash = format!("sha256:{}", "1".repeat(64));
        let source =
            json!({"narrativePackageSha256":hash,"sceneMapSha256":hash,"releaseBundleSha256":hash});
        let plan=seal(json!({"schemaVersion":"nvc.animatic-plan/1","source":source,"boardHash":hash,"boardReviewHash":hash,
            "fps":12,"canvas":{"width":64,"height":64,"background":"#000000"},
            "assets":[{"assetId":"actor","width":1,"height":1,"alpha":alpha_policy,"brief":"MECHANICAL_TEST_ONLY"}],
            "panels":[{"sceneId":"scene.test","panelId":"panel.test","durationFrames":1,
                "camera":[{"frame":0,"x":0,"y":0,"scalePerMille":1000,"easing":"linear"}],
                "layers":[{"layerId":"actor","depthPerMille":1000,"poses":[{"frame":0,"assetId":"actor"}],
                    "motion":[{"frame":0,"x":0,"y":0,"scalePerMille":1000,"easing":"linear","rotationMilliDeg":0,"opacityPerMille":1000}]}],
                "captions":[]}],"openQuestions":[]})).unwrap();
        let review=seal(json!({"schemaVersion":"nvc.animatic-review/1","source":source,"planHash":plan["contentHash"],
            "decision":"approved","reviewer":"SIMULATED_TEST_ONLY","note":"mechanical fixture","reviewedAt":"2026-10-03T00:00:00Z"})).unwrap();
        let packet = seal(
            json!({"schemaVersion":"nvc.animatic-packet/1","plan":plan,"review":review,
            "captions":[{"panelId":"panel.test","soundNotes":"no audio","lines":[]}]}),
        )
        .unwrap();
        std::fs::write(
            root.join("animatic/packet.json"),
            canonical(&packet).unwrap(),
        )
        .unwrap();
        let png = png::tests::png(alpha);
        std::fs::write(root.join("animatic/actor.png"), &png).unwrap();
        let draft = json!({"schemaVersion":"sac.animatic-assets/1","packetHash":packet["contentHash"],
            "assets":[{"assetId":"actor","path":"animatic/actor.png","sha256":format!("sha256:{}",sha256(&png)),
                "width":1,"height":1,"alpha":alpha_policy,"provenance":{"executor":"user-supplied","source":"MECHANICAL_TEST_ONLY"}}]});
        std::fs::write(root.join("draft.json"), canonical(&draft).unwrap()).unwrap();
        let options = Options {
            operation: "submit".into(),
            project_root: root,
            owner_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            packet: "animatic/packet.json".into(),
            draft: Some("draft.json".into()),
            pack: Some("animatic/assets.json".into()),
            output: Some("animatic/assets.json".into()),
            expected_hash: None,
            reviewer: None,
            decision: None,
            note: String::new(),
        };
        (temp, options, draft)
    }
    #[test]
    fn candidate_inspection_review_and_exact_retry_are_distinct() {
        let (_temp, mut options, _) = fixture(255, "opaque");
        let candidate = run(&options).unwrap();
        assert_eq!(candidate["status"], "candidate");
        let bytes = std::fs::read(options.project_root.join("animatic/assets.json")).unwrap();
        assert_eq!(run(&options).unwrap(), candidate);
        assert_eq!(
            std::fs::read(options.project_root.join("animatic/assets.json")).unwrap(),
            bytes
        );
        options.operation = "inspect".into();
        assert_eq!(run(&options).unwrap()["status"], "inspected");
        options.operation = "review".into();
        options.output = Some("animatic/review.json".into());
        options.expected_hash = Some(candidate["contentHash"].as_str().unwrap().into());
        options.reviewer = Some("SIMULATED_TEST_ONLY".into());
        options.decision = Some("approved".into());
        let review = run(&options).unwrap();
        assert_eq!(review["decision"], "approved");
        assert_eq!(run(&options).unwrap(), review);
        options.decision = Some("rejected".into());
        assert!(run(&options)
            .unwrap_err()
            .message
            .contains("IMMUTABLE_REVIEW_CONFLICT"));
    }
    #[test]
    fn review_hash_and_pixel_drift_are_refused_before_publication() {
        let (_temp, mut options, _) = fixture(255, "opaque");
        run(&options).unwrap();
        options.operation = "review".into();
        options.output = Some("animatic/review.json".into());
        options.expected_hash = Some(format!("sha256:{}", "0".repeat(64)));
        options.reviewer = Some("SIMULATED_TEST_ONLY".into());
        options.decision = Some("approved".into());
        assert!(run(&options)
            .unwrap_err()
            .message
            .contains("REVIEW_HASH_CHANGED"));
        assert!(!options.project_root.join("animatic/review.json").exists());
        std::fs::write(
            options.project_root.join("animatic/actor.png"),
            png::tests::png(128),
        )
        .unwrap();
        options.operation = "inspect".into();
        assert!(run(&options)
            .unwrap_err()
            .message
            .contains("PNG_HASH_CHANGED"));
    }
    #[test]
    fn opaque_cutout_and_entirely_transparent_checks_use_pixels() {
        for (alpha, policy, expected) in [
            (255, "cutout", "PNG_ALPHA_MISMATCH"),
            (0, "cutout", "PNG_ENTIRELY_TRANSPARENT"),
            (128, "opaque", "PNG_ALPHA_MISMATCH"),
        ] {
            let (_temp, options, _) = fixture(alpha, policy);
            assert!(run(&options).unwrap_err().message.contains(expected));
            assert!(!options.project_root.join("animatic/assets.json").exists());
        }
        let (_temp, options, _) = fixture(128, "cutout");
        assert_eq!(run(&options).unwrap()["status"], "candidate");
    }
    #[test]
    fn symlink_png_and_duplicate_asset_ids_are_refused() {
        let (_temp, options, mut draft) = fixture(255, "opaque");
        let path = options.project_root.join("animatic/actor.png");
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::write(options.project_root.join("outside.png"), bytes).unwrap();
        std::os::unix::fs::symlink("../outside.png", &path).unwrap();
        assert!(run(&options).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, png::tests::png(255)).unwrap();
        let duplicate = draft["assets"][0].clone();
        draft["assets"].as_array_mut().unwrap().push(duplicate);
        std::fs::write(
            options.project_root.join("draft.json"),
            canonical(&draft).unwrap(),
        )
        .unwrap();
        assert!(run(&options)
            .unwrap_err()
            .message
            .contains("ASSET_COVERAGE"));
    }
    #[test]
    fn sac_integer_schema_preserves_legacy_integral_float_dimension_bytes() {
        let (_temp, options, mut draft) = fixture(255, "opaque");
        draft["assets"][0]["width"] = json!(1.0);
        draft["assets"][0]["height"] = json!(1.0);
        std::fs::write(
            options.project_root.join("draft.json"),
            canonical(&draft).unwrap(),
        )
        .unwrap();
        let candidate = run(&options).unwrap();
        assert_eq!(candidate["status"], "candidate");
        let saved =
            std::fs::read_to_string(options.project_root.join("animatic/assets.json")).unwrap();
        assert!(saved.contains("\"width\":1.0"));
        assert!(saved.contains("\"height\":1.0"));
    }
}
