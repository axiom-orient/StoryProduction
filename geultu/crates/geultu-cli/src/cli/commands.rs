use geultu::{Application, Workspace};
use geultu_writer_process::ProcessWriter;
use geultu_writing::builtin_adapters;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};

use super::{
    parse_args, print_result, print_value, read_json_argument, read_text, require_len, usage,
    CliError, Options,
};
use crate::mcp;

pub(crate) fn run() -> Result<(), CliError> {
    let options = parse_args(std::env::args().skip(1).collect())?;
    validate_command(&options)?;
    if options.command == "help" {
        writeln!(io::stdout().lock(), "{}", usage())?;
        return Ok(());
    }
    if options.command == "version" {
        writeln!(io::stdout().lock(), "geultu {}", env!("CARGO_PKG_VERSION"))?;
        return Ok(());
    }
    if options.command == "init" {
        let workspace = Workspace::init(&options.workspace)?;
        print_value(
            json!({"ok":true,"result":{"workspace":workspace.root(),"schema_version":geultu::model::SCHEMA_VERSION}}),
            options.pretty,
        )?;
        return Ok(());
    }
    let app = open_application(&options)?;
    match options.command.as_str() {
        "tools" => {
            print_value(
                json!({"ok":true,"result":app.tool_definitions()}),
                options.pretty,
            )?;
        }
        "call" => {
            let name = &options.args[0];
            let input = options.args.get(1).map(String::as_str).unwrap_or("{}");
            let arguments = read_json_argument(input)?;
            print_result(app.call(name, arguments), options.pretty)?;
        }
        "mcp" => {
            mcp::run(app).map_err(CliError::Other)?;
        }
        "doctor" => {
            let meta = app.workspace().metadata()?;
            let projects = app.workspace().list_projects()?;
            print_value(
                json!({"ok":true,"result":{"engine_version":env!("CARGO_PKG_VERSION"),"workspace":app.workspace().root(),"schema_version":meta.schema_version,"project_count":projects.len(),"tool_count":app.tool_definitions().len(),"mcp_protocols":mcp::SUPPORTED_PROTOCOLS}}),
                options.pretty,
            )?;
        }
        "project-create" => {
            print_result(
                app.call(
                    "project.create",
                    json!({"project_id":options.args[0],"title":options.args[1]}),
                ),
                options.pretty,
            )?;
        }
        "project-list" => {
            print_result(app.call("project.list", json!({})), options.pretty)?;
        }
        "scene-add" => {
            let text = options
                .args
                .get(3)
                .map(|value| read_text(value))
                .transpose()?
                .unwrap_or_default();
            // T33 story.export requires scene.order >= 1; this CLI has no
            // order flag, so derive the next order from existing scenes.
            let current = app.workspace().load_project(&options.args[0])?;
            let max_order = current
                .scenes
                .values()
                .map(|scene| scene.order)
                .max()
                .unwrap_or(0)
                .max(0);
            let order = max_order
                .checked_add(1)
                .ok_or_else(|| CliError::Usage("scene order exceeds i64 capacity".into()))?;
            print_result(app.call("scene.add", json!({"project_id":options.args[0],"scene_id":options.args[1],"title":options.args[2],"text":text,"order":order})), options.pretty)?;
        }
        "scene-set" => {
            let text = read_text(&options.args[2])?;
            let expected = options
                .args
                .get(3)
                .map(|value| {
                    value.parse::<u64>().map_err(|_| {
                        CliError::Usage("expected scene revision must be an integer".into())
                    })
                })
                .transpose()?;
            print_result(app.call("scene.set_text", json!({"project_id":options.args[0],"scene_id":options.args[1],"text":text,"expected_scene_revision":expected})), options.pretty)?;
        }
        "review" => {
            print_result(
                app.call("review.run", json!({"project_id":options.args[0]})),
                options.pretty,
            )?;
        }
        "export" => {
            let value = app.call(
                "project.export_markdown",
                json!({"project_id":options.args[0]}),
            )?;
            let markdown = value
                .get("markdown")
                .and_then(Value::as_str)
                .ok_or_else(|| CliError::Other("export response omitted markdown".into()))?;
            if let Some(path) = options.args.get(1) {
                fs::write(path, markdown)?;
                print_value(
                    json!({"ok":true,"result":{"project_id":options.args[0],"output":path,"revision":value["revision"]}}),
                    options.pretty,
                )?;
            } else {
                io::stdout().lock().write_all(markdown.as_bytes())?;
            }
        }
        "export-reviewed-synopsis" => {
            let revision = options.args[2]
                .parse::<u64>()
                .map_err(|_| CliError::Usage("REVISION must be a positive integer".into()))?;
            let language = &options.args[4];
            let reason = options.args.get(5);
            print_result(
                app.call(
                    "synopsis.export_reviewed_handoff",
                    json!({
                        "project_id": options.args[0],
                        "actor_id": options.args[1],
                        "language": language,
                        "reason": reason,
                        "expected_source_revision": revision,
                        "expected_synopsis_sha256": options.args[3]
                    }),
                ),
                options.pretty,
            )?;
        }
        other => return Err(CliError::Usage(format!("unknown command '{other}'"))),
    }
    Ok(())
}

fn validate_command(options: &Options) -> Result<(), CliError> {
    let (minimum, maximum, usage) = match options.command.as_str() {
        "help" => (0, 0, "help"),
        "version" => (0, 0, "version"),
        "init" => (0, 0, "init"),
        "tools" => (0, 0, "tools"),
        "call" => (1, 2, "call TOOL_NAME [JSON|@FILE|-]"),
        "mcp" => (0, 0, "mcp"),
        "doctor" => (0, 0, "doctor"),
        "project-create" => (2, 2, "project-create PROJECT_ID TITLE"),
        "project-list" => (0, 0, "project-list"),
        "scene-add" => (3, 4, "scene-add PROJECT_ID SCENE_ID TITLE [TEXT_FILE|-]"),
        "scene-set" => (
            3,
            4,
            "scene-set PROJECT_ID SCENE_ID TEXT_FILE|- [EXPECTED_SCENE_REVISION]",
        ),
        "review" => (1, 1, "review PROJECT_ID"),
        "export" => (1, 2, "export PROJECT_ID [OUTPUT_FILE]"),
        "export-reviewed-synopsis" => (
            5,
            6,
            "export-reviewed-synopsis PROJECT_ID ACTOR_ID REVISION SHA256 LANGUAGE [REASON]",
        ),
        other => return Err(CliError::Usage(format!("unknown command '{other}'"))),
    };
    require_len(&options.args, minimum, maximum, usage)
}

fn open_application(options: &Options) -> Result<Application, CliError> {
    let mut generation = geultu::GenerationRegistry::default();
    for adapter in builtin_adapters() {
        generation.add_writing_adapter(adapter);
    }
    if let Some(program) = &options.writer_program {
        generation.add_provider(ProcessWriter::new(
            "process",
            program.clone(),
            options.writer_args.clone(),
        )?);
    }
    Ok(Application::open(&options.workspace)?.with_generation_registry(generation))
}
