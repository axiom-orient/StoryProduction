//! Bind generated image candidates to the existing SAC draft, without approval.
use crate::{
    contracts::SchemaRegistry,
    effects::{files::Directory, hash, json},
    error::{Error, Result},
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Prepared {
    pub draft: Value,
    captured: BTreeMap<String, Vec<u8>>,
}

fn capture(
    root: &Directory,
    path: &str,
    captured: &mut BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>> {
    let bytes = root.read_bytes_limited(path, 64 * 1024 * 1024)?;
    if captured.get(path).is_some_and(|old| old != &bytes) {
        return Err(Error::not_applied("handoff input changed while preparing"));
    }
    captured.insert(path.to_owned(), bytes.clone());
    Ok(bytes)
}

pub(super) fn prepare(
    root: &Directory,
    packet: &Value,
    bindings: &str,
    schemas: &SchemaRegistry,
) -> Result<Prepared> {
    let mut captured = BTreeMap::new();
    let mapping = json::parse_object(&capture(root, bindings, &mut captured)?)?;
    if mapping
        .as_object()
        .unwrap()
        .keys()
        .any(|key| key != "assets")
    {
        return Err(Error::not_applied("asset bindings accept only assets"));
    }
    let mappings = mapping["assets"]
        .as_array()
        .ok_or_else(|| Error::not_applied("asset bindings require an assets array"))?;
    let requirements = packet["plan"]["assets"]
        .as_array()
        .ok_or_else(|| Error::not_applied("packet assets missing"))?;
    let packet_hash = packet["contentHash"]
        .as_str()
        .filter(|hash| hash::is_sha256(hash))
        .ok_or_else(|| Error::not_applied("packet content hash missing"))?;
    let source = json!({"kind":"nvc.animatic-packet", "identity":format!("packet.{}", &packet_hash[7..]), "sha256":packet_hash});
    let mut seen = BTreeSet::new();
    let mut assets = Vec::new();
    for binding in mappings {
        let fields = binding
            .as_object()
            .ok_or_else(|| Error::not_applied("asset binding must be an object"))?;
        if fields.len() != 2 || !fields.contains_key("assetId") || !fields.contains_key("result") {
            return Err(Error::not_applied(
                "each binding requires exactly assetId and result",
            ));
        }
        let id = binding["assetId"]
            .as_str()
            .ok_or_else(|| Error::not_applied("assetId missing"))?;
        let required = requirements
            .iter()
            .find(|asset| asset["assetId"] == id)
            .ok_or_else(|| Error::not_applied("asset is not required by the current packet"))?;
        if !seen.insert(id.to_owned()) {
            return Err(Error::not_applied("duplicate asset binding"));
        }
        let result_path = binding["result"]
            .as_str()
            .ok_or_else(|| Error::not_applied("result path missing"))?;
        let result_bytes = capture(root, result_path, &mut captured)?;
        let result = json::parse_object(&result_bytes)?;
        schemas.validate(super::capability::RESULT_SCHEMA, &result)?;
        if result["status"] != "succeeded"
            || result["candidateOnly"] != true
            || result["capability"] != "image.generate"
            || result["provider"]["kind"] != "codex"
            || result["source"] != source
        {
            return Err(Error::not_applied(
                "image result must be a succeeded Codex candidate for this exact NVC packet",
            ));
        }
        let artifacts = result["artifacts"].as_array().unwrap();
        for artifact in artifacts {
            let path = artifact["path"]
                .as_str()
                .ok_or_else(|| Error::not_applied("artifact path missing"))?;
            let bytes = capture(root, path, &mut captured)?;
            if artifact["sha256"] != hash::sha256_bytes(&bytes)
                || artifact["byteLength"] != bytes.len() as u64
            {
                return Err(Error::not_applied(
                    "generated artifact hash or length changed",
                ));
            }
        }
        let images = artifacts
            .iter()
            .filter(|a| a["kind"] == "image_candidate" && a["mediaType"] == "image/png")
            .collect::<Vec<_>>();
        if images.len() != 1
            || images[0]["width"] != required["width"]
            || images[0]["height"] != required["height"]
        {
            return Err(Error::not_applied(
                "one PNG with the exact required dimensions is required per asset",
            ));
        }
        let receipt = &result["providerReceipt"];
        if receipt["backend"] != "codex-imagegen"
            || ["thread_id", "image_call_id"]
                .iter()
                .any(|key| receipt[key].as_str().is_none_or(str::is_empty))
            || receipt["sha256"].as_str() != images[0]["sha256"].as_str().map(|hash| &hash[7..])
        {
            return Err(Error::not_applied(
                "generation provenance is missing or does not match the image",
            ));
        }
        assets.push(json!({"assetId":id, "path":images[0]["path"], "sha256":images[0]["sha256"],
            "width":required["width"], "height":required["height"], "alpha":required["alpha"],
            "provenance":{"executor":"codex-imagegen", "source":format!("{}; result={}; request={}; thread={}; call={}",
                result_path, hash::sha256_bytes(&result_bytes), result["requestHash"].as_str().unwrap(), receipt["thread_id"].as_str().unwrap(), receipt["image_call_id"].as_str().unwrap())}}));
    }
    if assets.len() != requirements.len() {
        return Err(Error::not_applied(
            "bindings must cover every required asset exactly once",
        ));
    }
    Ok(Prepared {
        draft: json!({"schemaVersion":"sac.animatic-assets/1", "packetHash":packet_hash, "assets":assets}),
        captured,
    })
}

impl Prepared {
    pub fn revalidate(&self, root: &Directory) -> Result<()> {
        root.verify_identity()?;
        for (path, bytes) in &self.captured {
            if root.read_bytes_limited(path, 64 * 1024 * 1024)? != *bytes {
                return Err(Error::not_applied(
                    "handoff input changed before draft publication",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    #[test]
    fn handoff_binds_source_coverage_dimensions_provenance_and_current_bytes() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap();
        let schemas = SchemaRegistry::new(&repository.join("integration/contracts")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let selected_root = temp.path().canonicalize().unwrap();
        let root = Directory::open(&selected_root, false).unwrap();
        // A filesystem seam test, not provider output or visual qualification.
        let pixels = b"synthetic-boundary-bytes";
        fs::write(temp.path().join("image.png"), pixels).unwrap();
        let hash = format!("sha256:{}", "a".repeat(64));
        let packet = json!({"contentHash":hash, "plan":{"assets":[{"assetId":"shot-1", "width":32,"height":32,"alpha":"opaque"}]}});
        let mut result: Value = serde_json::from_slice(
            &fs::read(
                repository.join("integration/contracts/examples/capability-result.succeeded.json"),
            )
            .unwrap(),
        )
        .unwrap();
        result["capability"] = json!("image.generate");
        result["provider"]["kind"] = json!("codex");
        result["provider"]["networkPolicy"] = json!("required");
        result["source"] = json!({"kind":"nvc.animatic-packet", "identity":format!("packet.{}", "a".repeat(64)),"sha256":hash});
        result["artifacts"] = json!([{"kind":"image_candidate","identity":"image.test","path":"image.png","sha256":hash::sha256_bytes(pixels),"byteLength":pixels.len(),"mediaType":"image/png","width":32,"height":32}]);
        result["providerReceipt"] = json!({"backend":"codex-imagegen","thread_id":"synthetic.thread","image_call_id":"synthetic.call","sha256":hash::sha256_hex_bytes(pixels)});
        let mapping = json!({"assets":[{"assetId":"shot-1","result":"result.json"}]});
        fs::write(
            temp.path().join("bindings.json"),
            serde_json::to_vec(&mapping).unwrap(),
        )
        .unwrap();
        let save = |value: &Value| {
            fs::write(
                temp.path().join("result.json"),
                serde_json::to_vec(value).unwrap(),
            )
            .unwrap()
        };
        save(&result);
        let prepared = prepare(&root, &packet, "bindings.json", &schemas).unwrap();
        assert_eq!(prepared.draft["packetHash"], hash);
        assert!(prepared.draft.get("contentHash").is_none());
        prepared.revalidate(&root).unwrap();
        fs::write(temp.path().join("image.png"), b"changed").unwrap();
        assert!(prepared.revalidate(&root).is_err());
        assert!(prepare(&root, &packet, "bindings.json", &schemas).is_err());
        fs::write(temp.path().join("image.png"), pixels).unwrap();
        for (path, wrong) in [
            (vec!["status"], json!("outcome_unknown")),
            (
                vec!["source", "sha256"],
                json!(format!("sha256:{}", "b".repeat(64))),
            ),
            (vec!["providerReceipt", "sha256"], json!("b".repeat(64))),
        ] {
            let mut bad = result.clone();
            let mut field = &mut bad;
            for key in path {
                field = &mut field[key];
            }
            *field = wrong;
            save(&bad);
            assert!(prepare(&root, &packet, "bindings.json", &schemas).is_err());
        }
        save(&result);
        let mut bad_packet = packet.clone();
        bad_packet["plan"]["assets"][0]["width"] = json!(64);
        assert!(prepare(&root, &bad_packet, "bindings.json", &schemas).is_err());
        fs::write(temp.path().join("bindings.json"), br#"{"assets":[]}"#).unwrap();
        assert!(prepare(&root, &packet, "bindings.json", &schemas).is_err());
    }
}
