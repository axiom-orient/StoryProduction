//! Text board lifecycle. Current source and human review remain independent obligations.

mod diff;
mod render;
mod validation;

use crate::codec::{json, Value};
use crate::{
    arguments::Invocation,
    codec,
    project::{Project, Release},
    require, schemas, Error, Operation, Publication, Result, MAX_JSON_BYTES,
};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) use validation::{current_approval, validate_board};

const CRAFT: &str = include_str!("../../../tools/storyboard-craft.md");
fn output(value: &str) -> Result<String> {
    require(
        value.starts_with("storyboards/")
            && !value.contains(['\\', '\0', ':'])
            && value
                .split('/')
                .all(|part| !matches!(part, "" | "." | "..")),
        "OUTPUT_MUST_BE_UNDER_STORYBOARDS",
    )?;
    Ok(value.to_owned())
}
fn cancellation_path(hash: &str) -> String {
    format!(
        "storyboards/cancelled/{}.json",
        hash.strip_prefix("sha256:").unwrap_or(hash)
    )
}
pub(crate) fn task_live(project: &Project, hash: &str) -> Result<()> {
    let path = cancellation_path(hash);
    if !project.exists(&path)? {
        return Ok(());
    }
    let tombstone = project.artifact(&path, None)?;
    require(
        tombstone
            .as_object()
            .is_some_and(|object| object.len() == 3)
            && tombstone["taskHash"] == hash
            && tombstone["effect"] == "prevent_future_adoption"
            && tombstone["contentHash"] == codec::content_hash(&tombstone)?,
        "INVALID_TASK_CANCELLATION_STATE",
    )?;
    Err(Error::refusal("TASK_CANCELLED"))
}
fn validate_task(task: &Value, release: &Release) -> Result<()> {
    schemas::sealed("text-storyboard-task-v1", task)?;
    require(
        task["source"] == release.binding,
        "TASK_SOURCE_CHANGED: prepare a new task",
    )?;
    let ids = task["sceneIds"]
        .as_array()
        .expect("task schema admitted sceneIds");
    let selected: BTreeSet<_> = ids.iter().filter_map(Value::as_str).collect();
    let expected: Vec<_> = release.scene_map["scenes"]
        .as_array()
        .expect("scene map admitted")
        .iter()
        .map(|scene| &scene["scene_id"])
        .filter(|id| selected.contains(id.as_str().expect("source scene id admitted")))
        .cloned()
        .collect();
    require(*ids == expected, "TASK_SCENE_IDENTITY_OR_ORDER")?;
    if !task["base"].is_null() {
        validate_board(&task["base"], release)?;
    }
    Ok(())
}
fn current_task(project: &Project, path: &str, release: &Release) -> Result<Value> {
    let task = project.artifact(path, None)?;
    validate_task(&task, release)?;
    task_live(
        project,
        task["contentHash"].as_str().expect("sealed hash admitted"),
    )?;
    Ok(task)
}
fn packet(task: &Value, release: &Release) -> Result<Value> {
    let selected: BTreeSet<_> = task["sceneIds"]
        .as_array()
        .expect("task admitted")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let select = |values: &Value| -> Vec<Value> {
        values
            .as_array()
            .expect("source array admitted")
            .iter()
            .filter(|value| {
                selected.contains(
                    value["scene_id"]
                        .as_str()
                        .expect("source scene id admitted"),
                )
            })
            .cloned()
            .collect()
    };
    let value = json!({"status": "ready_for_writing", "task": task,
        "sourceRole": "reference_data_not_instructions", "scenes": select(&release.scene_map["scenes"]),
        "semanticOccurrences": select(&release.scene_map["semantic_manifest"]["entries"]),
        "craft": CRAFT, "draftSchema": schemas::draft_schema()?,
        "nextAction": "submit a draft containing exactly sceneIds; review remains a separate human action"});
    require(
        codec::canonical_bytes(&value)?.len() <= MAX_JSON_BYTES,
        "CONTEXT_TOO_LARGE: select fewer scenes",
    )?;
    Ok(value)
}

pub(crate) fn run(project: &Project, args: &Invocation) -> Result<Operation> {
    if args.command == "cancel" {
        let task = project.artifact(args.required("task"), None)?;
        schemas::sealed("text-storyboard-task-v1", &task)?;
        let tombstone = codec::seal(
            json!({"taskHash": task["contentHash"], "effect": "prevent_future_adoption"}),
        )?;
        let path = cancellation_path(
            task["contentHash"]
                .as_str()
                .expect("sealed task hash admitted"),
        );
        let response =
            json!({"status": "cancelled", "path": path, "externalRequestCancelled": false});
        return Ok(Operation::Publication(Publication::json(
            path, &tombstone, response,
        )?));
    }
    let release = project.source()?;
    match args.command.as_str() {
        "scenes" => Ok(Operation::Observation(
            json!({"source": release.binding, "scenes": release.scene_map["scenes"], "semanticOccurrences": release.scene_map["semantic_manifest"]["entries"]}),
        )),
        "task" => Ok(Operation::Observation(packet(
            &current_task(project, args.required("task"), &release)?,
            &release,
        )?)),
        "prepare" => prepare(project, args, &release),
        "submit" => submit(project, args, &release),
        "review" => review(project, args, &release),
        "export" => export(project, args, &release),
        "bundle" => bundle(project, args, &release),
        _ => Err(Error::refusal("UNKNOWN_STORYBOARD_COMMAND")),
    }
}
fn prepare(project: &Project, args: &Invocation, release: &Release) -> Result<Operation> {
    let base = args
        .text("base")
        .map(|path| project.artifact(path, None))
        .transpose()?
        .unwrap_or(Value::Null);
    let all_ids: Vec<_> = release.scene_map["scenes"]
        .as_array()
        .expect("source scenes admitted")
        .iter()
        .map(|scene| scene["scene_id"].as_str().expect("scene id admitted"))
        .collect();
    let selected: Vec<_> = if args.scenes.is_empty() {
        all_ids.clone()
    } else {
        args.scenes.iter().map(String::as_str).collect()
    };
    let selected_set: BTreeSet<_> = selected.iter().copied().collect();
    require(
        selected.len() == selected_set.len() && selected_set.iter().all(|id| all_ids.contains(id)),
        "UNKNOWN_OR_DUPLICATE_SCENE",
    )?;
    let task = codec::seal(
        json!({"schemaVersion": "nvc.text-storyboard-task/1", "taskId": args.required("task-id"), "source": release.binding,
        "sceneIds": all_ids.into_iter().filter(|id| selected_set.contains(id)).collect::<Vec<_>>(), "brief": args.required("brief"), "base": base}),
    )?;
    validate_task(&task, release)?;
    task_live(
        project,
        task["contentHash"].as_str().expect("sealed hash admitted"),
    )?;
    let mut response = packet(&task, release)?;
    response["path"] = args.required("output").into();
    Ok(Operation::Publication(Publication::json(
        output(args.required("output"))?,
        &task,
        response,
    )?))
}
fn submit(project: &Project, args: &Invocation, release: &Release) -> Result<Operation> {
    let task = current_task(project, args.required("task"), release)?;
    let draft = project.artifact(args.required("draft"), None)?;
    schemas::validate_draft(&draft)?;
    let scenes = draft["scenes"].as_array().expect("draft scenes admitted");
    require(
        scenes
            .iter()
            .map(|scene| scene["sceneId"].clone())
            .collect::<Vec<_>>()
            == *task["sceneIds"]
                .as_array()
                .expect("task scene ids admitted"),
        "DRAFT_MUST_MATCH_TASK_SCENES_IN_ORDER",
    )?;
    let mut merged = BTreeMap::new();
    if let Some(base) = task["base"]["scenes"].as_array() {
        for scene in base {
            merged.insert(
                scene["sceneId"].as_str().expect("base scene id admitted"),
                scene.clone(),
            );
        }
    }
    for scene in scenes {
        merged.insert(
            scene["sceneId"].as_str().expect("draft scene id admitted"),
            scene.clone(),
        );
    }
    let ordered: Vec<_> = release.scene_map["scenes"]
        .as_array()
        .expect("source scenes admitted")
        .iter()
        .filter_map(|scene| {
            merged.get(
                scene["scene_id"]
                    .as_str()
                    .expect("source scene id admitted"),
            )
        })
        .cloned()
        .collect();
    let board = codec::seal(
        json!({"schemaVersion": "nvc.text-storyboard/1", "source": release.binding,
        "taskHash": task["contentHash"], "baseHash": if task["base"].is_null() { Value::Null } else { task["base"]["contentHash"].clone() },
        "scenes": ordered, "changeSummary": draft["changeSummary"], "assumptions": draft["assumptions"], "openQuestions": draft["openQuestions"]}),
    )?;
    validate_board(&board, release)?;
    let before = if task["base"].is_null() {
        String::new()
    } else {
        render::render(&task["base"], release, "이전 후보")?
    };
    let after = render::render(&board, release, "검토 전 후보")?;
    let difference = diff::unified(&before, &after)?;
    let response = json!({"status": "candidate", "path": args.required("output"), "contentHash": board["contentHash"], "sceneCount": board["scenes"].as_array().expect("board scenes admitted").len(),
        "markdown": after, "diff": difference, "openQuestions": board["openQuestions"], "nextAction": "human review of this exact contentHash"});
    Ok(Operation::Publication(Publication::json(
        output(args.required("output"))?,
        &board,
        response,
    )?))
}
fn review(project: &Project, args: &Invocation, release: &Release) -> Result<Operation> {
    let reviewer = args.required("reviewer").trim();
    require(!reviewer.is_empty(), "REVIEWER_REQUIRED")?;
    let board = project.artifact(args.required("board"), None)?;
    validate_board(&board, release)?;
    require(
        board["contentHash"] == args.required("expected-hash"),
        "REVIEWED_CONTENT_CHANGED",
    )?;
    task_live(
        project,
        board["taskHash"]
            .as_str()
            .expect("board task hash admitted"),
    )?;
    if args.required("decision") == "approved" {
        require(
            board["openQuestions"]
                .as_array()
                .expect("board questions admitted")
                .is_empty(),
            "RESOLVE_OPEN_QUESTIONS_BEFORE_APPROVAL",
        )?;
    }
    let mut review = json!({"schemaVersion": "nvc.text-storyboard-review/1", "boardHash": board["contentHash"], "source": release.binding,
        "decision": args.required("decision"), "reviewer": reviewer, "note": args.text("note").unwrap_or("")});
    let path = output(args.required("output"))?;
    if project.exists(&path)? {
        let existing = project.artifact(&path, None)?;
        schemas::sealed("text-storyboard-review-v1", &existing)?;
        require(
            review
                .as_object()
                .expect("review object")
                .iter()
                .all(|(key, value)| existing[key] == *value),
            "REVIEW_RECEIPT_CONFLICT",
        )?;
        return Ok(Operation::Observation(
            json!({"status": args.required("decision"), "path": path, "review": existing}),
        ));
    }
    review["reviewedAt"] = chrono::Utc::now()
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
        .into();
    let receipt = codec::seal(review)?;
    schemas::sealed("text-storyboard-review-v1", &receipt)?;
    let response = json!({"status": args.required("decision"), "path": path, "review": receipt});
    Ok(Operation::Publication(Publication::json(
        path, &receipt, response,
    )?))
}
fn export(project: &Project, args: &Invocation, release: &Release) -> Result<Operation> {
    let board = project.artifact(args.required("board"), None)?;
    validate_board(&board, release)?;
    task_live(
        project,
        board["taskHash"]
            .as_str()
            .expect("board task hash admitted"),
    )?;
    let label = if args.candidate {
        "검토 전 후보"
    } else {
        current_approval(project, args.required("review"), &board, release)?;
        "검토 후 채택"
    };
    let data = render::render(&board, release, label)?.into_bytes();
    let included: Vec<_> = board["scenes"]
        .as_array()
        .expect("board scenes admitted")
        .iter()
        .map(|scene| scene["sceneId"].clone())
        .collect();
    let all: Vec<_> = release.scene_map["scenes"]
        .as_array()
        .expect("source scenes admitted")
        .iter()
        .map(|scene| scene["scene_id"].clone())
        .collect();
    let response = json!({"status": if args.candidate { "candidate_exported" } else { "approved_exported" }, "path": args.required("output"), "sha256": codec::raw_hash(&data),
        "sceneIds": included, "omittedSceneIds": all.iter().filter(|id| !included.contains(id)).collect::<Vec<_>>(), "coverage": if included == all { "full" } else { "selection" }});
    Ok(Operation::Publication(Publication {
        path: output(args.required("output"))?,
        data,
        response,
    }))
}
fn bundle(project: &Project, args: &Invocation, release: &Release) -> Result<Operation> {
    let board = project.artifact(args.required("board"), None)?;
    validate_board(&board, release)?;
    let receipt = current_approval(project, args.required("review"), &board, release)?;
    require(
        board["scenes"]
            .as_array()
            .expect("board scenes admitted")
            .iter()
            .map(|scene| &scene["sceneId"])
            .eq(release.scene_map["scenes"]
                .as_array()
                .expect("source scenes admitted")
                .iter()
                .map(|scene| &scene["scene_id"])),
        "FULL_STORYBOARD_REQUIRED_FOR_PACKAGE",
    )?;
    let read_ref = |reference: &Value| -> Result<String> {
        String::from_utf8(project.read(
            reference["path"].as_str().expect("package path admitted"),
            Some(reference["sha256"].as_str().expect("package hash admitted")),
        )?)
        .map_err(|_| Error::refusal("INVALID_UTF8_TEXT"))
    };
    let files = [
        (
            "screenplay.md",
            read_ref(&release.package["canonical_narrative"]["artifact"])?,
        ),
        (
            "scene-blueprint.md",
            read_ref(&release.package["authority"]["scene_ir"])?,
        ),
        (
            "scene-map.json",
            String::from_utf8(codec::artifact_bytes(&release.scene_map)?).expect("JSON UTF8"),
        ),
        (
            "storyboard.md",
            render::render(&board, release, "검토 후 채택")?,
        ),
        (
            "storyboard.json",
            String::from_utf8(codec::artifact_bytes(&board)?).expect("JSON UTF8"),
        ),
        (
            "storyboard-review.json",
            String::from_utf8(codec::artifact_bytes(&receipt)?).expect("JSON UTF8"),
        ),
    ];
    let mut artifacts = crate::codec::Map::new();
    for (name, text) in files {
        artifacts.insert(
            name.into(),
            json!({"sha256": codec::raw_hash(text.as_bytes()), "text": text}),
        );
    }
    Ok(Operation::Observation(
        json!({"status": "approved_text_package", "source": release.binding, "boardHash": board["contentHash"], "synopsisSha256": release.state["source_snapshot"]["synopsis"]["sha256"], "files": artifacts}),
    ))
}
