//! Frame-addressable direction of the exact current approved board.
//! Plan candidate, human review, and exported packet have separate identities.

#[path = "caption_cues.rs"]
mod caption_cues;

use crate::codec::{json, Value};
use crate::{
    arguments::Invocation,
    codec,
    project::{Project, Release},
    require, schemas, storyboard, Error, Operation, Publication, Result,
};
use std::collections::{BTreeMap, BTreeSet};

fn output(value: &str) -> Result<String> {
    require(
        value.starts_with("animatic/")
            && !value.contains(['\\', '\0', ':'])
            && value
                .split('/')
                .all(|part| !matches!(part, "" | "." | "..")),
        "ANIMATIC_OUTPUT_PATH",
    )?;
    Ok(value.to_owned())
}
fn publication(path: &str, value: &Value, response: Value) -> Result<Operation> {
    let mut data = codec::canonical_bytes(value)?;
    data.push(b'\n');
    Ok(Operation::Publication(Publication {
        path: output(path)?,
        data,
        response,
    }))
}
fn validate_plan(
    plan: &Value,
    board: &Value,
    board_review: &Value,
    release: &Release,
) -> Result<()> {
    schemas::sealed("animatic-plan-v1", plan)?;
    require(plan["source"] == release.binding, "ANIMATIC_SOURCE_CHANGED")?;
    require(
        plan["boardHash"] == board["contentHash"]
            && plan["boardReviewHash"] == board_review["contentHash"],
        "ANIMATIC_BOARD_CHANGED",
    )?;
    let original_panels: Vec<_> = board["scenes"]
        .as_array()
        .expect("board scenes admitted")
        .iter()
        .flat_map(|scene| {
            scene["panels"]
                .as_array()
                .expect("board panels admitted")
                .iter()
                .map(move |panel| (&scene["sceneId"], panel))
        })
        .collect();
    let panels = plan["panels"].as_array().expect("plan panels admitted");
    require(
        panels
            .iter()
            .map(|panel| (&panel["sceneId"], &panel["panelId"]))
            .eq(original_panels
                .iter()
                .map(|(scene, panel)| (*scene, &panel["panelId"]))),
        "ANIMATIC_PANEL_COVERAGE_ORDER",
    )?;
    let mut used_assets = BTreeSet::new();
    let mut total = 0_u64;
    for (directed, (_, original)) in panels.iter().zip(&original_panels) {
        let duration = directed["durationFrames"]
            .as_u64()
            .expect("duration admitted");
        total += duration;
        let allowed: BTreeSet<_> = original["sourceRefs"]
            .as_array()
            .expect("source refs admitted")
            .iter()
            .map(|reference| {
                reference["semanticId"]
                    .as_str()
                    .expect("semantic id admitted")
            })
            .collect();
        let captions = directed["captions"].as_array().expect("captions admitted");
        let unique: BTreeSet<_> = captions
            .iter()
            .map(|caption| caption.as_str().expect("caption id admitted"))
            .collect();
        require(
            captions.len() == unique.len() && unique.is_subset(&allowed),
            "CAPTION_OUTSIDE_PANEL",
        )?;
        let layers = directed["layers"].as_array().expect("layers admitted");
        let ids: BTreeSet<_> = layers
            .iter()
            .map(|layer| layer["layerId"].as_str().expect("layer id admitted"))
            .collect();
        require(ids.len() == layers.len(), "DUPLICATE_LAYER_ID")?;
        for track in
            std::iter::once(&directed["camera"]).chain(layers.iter().map(|layer| &layer["motion"]))
        {
            let frames: Vec<_> = track
                .as_array()
                .expect("track admitted")
                .iter()
                .map(|frame| frame["frame"].as_u64().expect("frame admitted"))
                .collect();
            require(
                frames.windows(2).all(|pair| pair[0] < pair[1])
                    && frames.first() == Some(&0)
                    && frames.last() == Some(&(duration - 1)),
                "KEYFRAME_ENDPOINTS_ORDER",
            )?;
        }
        for layer in layers {
            let poses = layer["poses"].as_array().expect("poses admitted");
            let frames: Vec<_> = poses
                .iter()
                .map(|pose| pose["frame"].as_u64().expect("frame admitted"))
                .collect();
            require(
                frames.windows(2).all(|pair| pair[0] < pair[1])
                    && frames.first() == Some(&0)
                    && frames.last().is_some_and(|frame| *frame < duration),
                "POSE_FRAME_ORDER",
            )?;
            used_assets.extend(
                poses
                    .iter()
                    .map(|pose| pose["assetId"].as_str().expect("asset id admitted")),
            );
        }
    }
    require(
        total <= plan["fps"].as_u64().expect("fps admitted") * 600,
        "ANIMATIC_EXCEEDS_TEN_MINUTES",
    )?;
    let assets = plan["assets"].as_array().expect("assets admitted");
    let ids: BTreeSet<_> = assets
        .iter()
        .map(|asset| asset["assetId"].as_str().expect("asset id admitted"))
        .collect();
    require(
        assets.len() == ids.len() && ids == used_assets,
        "ASSET_COVERAGE",
    )?;
    caption_cues::validate(plan, release)?;
    require(
        plan["canvas"]["width"]
            .as_u64()
            .expect("width admitted")
            .is_multiple_of(2)
            && plan["canvas"]["height"]
                .as_u64()
                .expect("height admitted")
                .is_multiple_of(2),
        "VP9_REQUIRES_EVEN_CANVAS",
    )
}

pub(crate) fn run(project: &Project, args: &Invocation) -> Result<Operation> {
    let release = project.source()?;
    let board = project.artifact(args.required("board"), None)?;
    storyboard::validate_board(&board, &release)?;
    let board_review =
        storyboard::current_approval(project, args.required("board-review"), &board, &release)?;
    if args.command == "source" {
        return Ok(Operation::Observation(
            json!({"source": release.binding, "board": board, "boardReview": board_review, "semanticOccurrences": release.scene_map["semantic_manifest"]["entries"]}),
        ));
    }
    let mut plan = project.artifact(
        args.required(if args.command == "submit" {
            "draft"
        } else {
            "plan"
        }),
        None,
    )?;
    if args.command == "submit" {
        require(
            plan.get("contentHash").is_none(),
            "DRAFT_MUST_NOT_BE_SEALED",
        )?;
        plan = codec::seal(plan)?;
    }
    validate_plan(&plan, &board, &board_review, &release)?;
    match args.command.as_str() {
        "submit" => publication(
            args.required("output"),
            &plan,
            json!({"status": "candidate", "path": args.required("output"), "contentHash": plan["contentHash"]}),
        ),
        "review" => {
            require(
                plan["contentHash"] == args.required("expected-hash"),
                "REVIEW_HASH_CHANGED",
            )?;
            require(
                !args.required("reviewer").trim().is_empty(),
                "REVIEWER_REQUIRED",
            )?;
            require(
                plan["openQuestions"]
                    .as_array()
                    .expect("plan questions admitted")
                    .is_empty()
                    || args.required("decision") == "rejected",
                "UNRESOLVED_DIRECTION",
            )?;
            let mut receipt = codec::seal(
                json!({"schemaVersion": "nvc.animatic-review/1", "planHash": plan["contentHash"], "source": release.binding,
                "decision": args.required("decision"), "reviewer": args.required("reviewer"), "note": args.text("note").unwrap_or(""),
                "reviewedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false)}),
            )?;
            schemas::sealed("animatic-review-v1", &receipt)?;
            let path = output(args.required("output"))?;
            if project.exists(&path)? {
                let existing = project.artifact(&path, None)?;
                schemas::sealed("animatic-review-v1", &existing)?;
                require(
                    receipt
                        .as_object()
                        .expect("receipt object")
                        .iter()
                        .filter(|(key, _)| !matches!(key.as_str(), "reviewedAt" | "contentHash"))
                        .all(|(key, value)| existing[key] == *value),
                    "IMMUTABLE_REVIEW_CONFLICT",
                )?;
                receipt = existing;
            }
            publication(&path, &receipt, receipt.clone())
        }
        "export" => {
            let review = project.artifact(args.required("review"), None)?;
            schemas::sealed("animatic-review-v1", &review)?;
            require(
                review["planHash"] == plan["contentHash"]
                    && review["source"] == release.binding
                    && review["decision"] == "approved",
                "DIRECTION_NOT_APPROVED",
            )?;
            let semantics: BTreeMap<_, _> = release.scene_map["semantic_manifest"]["entries"]
                .as_array()
                .expect("semantics admitted")
                .iter()
                .map(|entry| {
                    (
                        entry["semantic_id"].as_str().expect("semantic id admitted"),
                        entry,
                    )
                })
                .collect();
            let original_panels: BTreeMap<_, _> = board["scenes"]
                .as_array()
                .expect("board scenes admitted")
                .iter()
                .flat_map(|scene| {
                    scene["panels"]
                        .as_array()
                        .expect("board panels admitted")
                        .iter()
                        .map(|panel| (panel["panelId"].as_str().expect("panel id admitted"), panel))
                })
                .collect();
            let captions: Vec<_> = plan["panels"].as_array().expect("plan panels admitted").iter().map(|panel| {
                let original = original_panels[panel["panelId"].as_str().expect("panel id admitted")];
                let lines: Vec<_> = panel["captions"].as_array().expect("caption ids admitted").iter().map(|key| {
                    let entry = semantics[key.as_str().expect("semantic id admitted")];
                    json!({"semanticId": key, "entryHash": entry["entry_hash"], "text": entry["source_text"]})
                }).collect();
                json!({"panelId": panel["panelId"], "soundNotes": original["soundNotes"], "lines": lines})
            }).collect();
            let packet = codec::seal(
                json!({"schemaVersion": "nvc.animatic-packet/1", "plan": plan, "review": review, "captions": captions}),
            )?;
            schemas::sealed("animatic-packet-v1", &packet)?;
            if let Some(path) = args.text("output") {
                publication(path, &packet, packet.clone())
            } else {
                Ok(Operation::Observation(packet))
            }
        }
        _ => Err(Error::refusal("UNKNOWN_ANIMATIC_COMMAND")),
    }
}
