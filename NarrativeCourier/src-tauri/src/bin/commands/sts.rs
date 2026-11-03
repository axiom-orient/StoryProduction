//! STS-owned import, workflow, review, and screenplay interchange CLI commands.

use super::{write_json, write_text, CliResult, Command, Invocation};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use sts_workflow::{
    CreateReviewedSynopsisProjectRequest, Engine, ImportScreenplayRequest, NextPacket,
};

fn read_agent_artifact(
    engine: &Engine,
    project_name: &str,
    relative: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let relative_path = Path::new(relative);
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err("ARTIFACT_FILE must be a relative path without traversal".into());
    }
    let project = engine.project_dir(project_name)?;
    let project_meta = fs::symlink_metadata(&project)?;
    if project_meta.file_type().is_symlink() || !project_meta.is_dir() {
        return Err("project root is not a regular directory".into());
    }
    let real_project = fs::canonicalize(&project)?;
    let mut candidate = project;
    let parts: Vec<_> = relative_path.components().collect();
    if parts.is_empty() {
        return Err("ARTIFACT_FILE must name a regular file".into());
    }
    for (index, component) in parts.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err("ARTIFACT_FILE must contain only normal relative path components".into());
        };
        candidate.push(name);
        let metadata = fs::symlink_metadata(&candidate)?;
        if metadata.file_type().is_symlink() {
            return Err("ARTIFACT_FILE cannot contain symlink components".into());
        }
        if index + 1 == parts.len() {
            if !metadata.is_file() {
                return Err("ARTIFACT_FILE must be a regular file".into());
            }
        } else if !metadata.is_dir() {
            return Err("ARTIFACT_FILE parent must be a regular directory".into());
        }
    }
    let real_candidate = fs::canonicalize(&candidate)?;
    if !real_candidate.starts_with(&real_project) {
        return Err("ARTIFACT_FILE escapes the project root".into());
    }
    let final_meta = fs::symlink_metadata(&real_candidate)?;
    if final_meta.file_type().is_symlink() || !final_meta.is_file() {
        return Err("ARTIFACT_FILE must be a regular non-symlink file".into());
    }
    Ok(fs::read_to_string(if candidate.is_relative() {
        candidate
    } else {
        real_candidate
    })?)
}

pub(super) fn dispatch(invocation: &Invocation, engine: &Engine) -> CliResult<()> {
    let name = &invocation.project;
    let positional = &invocation.positional;
    match invocation.command {
        Command::Artifact => write_json(&engine.read_artifact(name, &positional[0])?),
        Command::SourceBinding => {
            let config = engine.load_config(&engine.project_dir(name)?)?;
            write_json(
                &serde_json::json!({"project_name": name, "reviewed_synopsis": config.reviewed_synopsis}),
            )
        }
        Command::Run => write_json(&engine.run_to_human_codex(name)?),
        Command::Cancel => {
            engine.cancel_codex(name)?;
            write_text(&format!("cancel requested for {name}\n"))
        }
        Command::Next => match engine.next(name)? {
            NextPacket::AgentTask { .. } => {
                let (_, text) = engine.task_text(name)?;
                write_text(&text)
            }
            other => write_json(&other),
        },
        Command::AgentTask => write_json(&engine.next(name)?),
        Command::Submit => {
            let artifact = read_agent_artifact(engine, name, &positional[0])?;
            write_json(&engine.submit(
                name,
                &artifact,
                invocation.required_text("--task-token")?,
            )?)
        }
        Command::Decide => write_json(&engine.decide(
            name,
            &positional[0],
            invocation.step(),
            invocation.text("--feedback"),
            invocation.required_text("--gate-token")?,
        )?),
        Command::Doctor => write_json(&engine.doctor(name)?),
        Command::Status => write_json(&engine.project_view(name)?),
        Command::Resume => {
            engine.resume(name)?;
            write_json(&engine.project_view(name)?)
        }
        Command::Export | Command::ExportFountain | Command::ExportFdx => {
            let extension = match invocation.command {
                Command::ExportFountain => "fountain",
                Command::ExportFdx => "fdx",
                _ => "docx",
            };
            let out = match positional.first() {
                Some(path) => PathBuf::from(path),
                None => engine
                    .project_dir(name)?
                    .join(format!("output/final_screenplay.{extension}")),
            };
            match invocation.command {
                Command::ExportFountain => {
                    engine.export_screenplay_fountain(name, &out)?;
                }
                Command::ExportFdx => {
                    engine.export_screenplay_fdx(name, &out)?;
                }
                _ => {
                    engine.export_docx(name, &out)?;
                }
            }
            write_text(&format!("{}\n", out.display()))
        }
        Command::Import => {
            let script = invocation.required_text("--script")?;
            let synopsis_file = invocation.required_text("--synopsis")?;
            let title = invocation
                .text("--title")
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    Path::new(script)
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("imported screenplay")
                        .to_owned()
                });
            write_json(&engine.import_screenplay(&ImportScreenplayRequest {
                project_name: name.clone(),
                title,
                synopsis: fs::read_to_string(synopsis_file)?,
                source_format: "original".into(),
                target_duration: invocation.duration(),
                genre: vec![],
                tone: String::new(),
                theme_direction: String::new(),
                script_path: script.to_owned(),
            })?)
        }
        Command::ImportReviewedSynopsis => {
            let root = match invocation.text("--handoff-root") {
                Some(path) => PathBuf::from(path),
                None => env::current_dir()?,
            };
            let root = root
                .into_os_string()
                .into_string()
                .map_err(|_| "handoff root must be UTF-8 for the public handoff request")?;
            write_json(&engine.create_project_from_reviewed_synopsis(
                &CreateReviewedSynopsisProjectRequest {
                    project_name: name.clone(),
                    title: invocation.text("--title").map(str::to_owned),
                    handoff_root: root,
                    handoff_path: invocation.required_text("--handoff")?.to_owned(),
                    target_duration: invocation.duration(),
                    genre: vec![],
                    tone: String::new(),
                    theme_direction: String::new(),
                },
            )?)
        }
        _ => Err("delivery command reached STS CLI owner".into()),
    }
}
