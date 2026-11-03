use narrative_courier_app_config::resolve_roots;
use std::env;
use sts_workflow::Engine;
mod commands;

fn main() {
    let arguments: Vec<_> = env::args().skip(1).collect();
    if let Some(lane @ ("storyboard" | "animatic")) = arguments.first().map(String::as_str) {
        std::process::exit(narrative_courier_authoring::cli_main(
            lane,
            arguments[1..].to_vec(),
        ));
    }
    if let Err(e) = real_main() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(invocation) = commands::parse(env::args().skip(1))? else {
        print_help();
        return Ok(());
    };
    let roots = resolve_roots(None, None)?;
    let _selected;
    let engine = if let Some(root) = invocation.text("--project-root") {
        _selected = Some(
            narrative_courier_runtime::working_directory::SelectedWorkingDirectory::enter(
                std::path::Path::new(root),
            )?,
        );
        Engine::with_selected_project(roots.assets, &invocation.project)?
    } else if env::var_os("STS_PROJECTS_ROOT").is_some_and(|root| root == ".") {
        _selected = Some(
            narrative_courier_runtime::working_directory::SelectedWorkingDirectory::enter(
                std::path::Path::new("."),
            )?,
        );
        Engine::with_working_projects_root(roots.assets)?
    } else {
        _selected = None;
        Engine::with_roots(roots.assets, roots.projects)?
    };
    commands::dispatch(&invocation, &engine)
}

fn print_help() {
    eprintln!(
        "sts — deterministic screenplay workflow\n\n\
  storyboard OP --project-root DIR  source-bound text boards; use storyboard --help\n\
  animatic OP --project-root DIR    reviewed frame direction; use animatic --help\n\n\
  sts run PROJECT       run Codex until the next human gate\n\
  sts cancel PROJECT    cancel the active Codex run\n\
  sts next PROJECT      inspect the next obligation\n\
  sts agent-task PROJECT  inspect the next obligation as JSON (including agent task token)\n\
  sts source-binding PROJECT  read the imported synopsis identity as JSON\n\
  sts artifact PROJECT NODE  read a stored STS artifact as JSON\n\
  sts submit PROJECT ARTIFACT_FILE --task-token TOKEN\n\
                     (ARTIFACT_FILE is relative to PROJECT and must be a regular file)\n\
  sts decide PROJECT approve|revise|pause --gate-token TOKEN [--step N] [--feedback TEXT]\n\
  sts doctor PROJECT    verify project integrity\n\
  sts status PROJECT    print UI/runtime state (including the review gate token)\n\
  sts resume PROJECT    resume a paused project\n\
  sts export PROJECT [PATH]\n\
  sts export-fountain PROJECT [PATH]\n\
  sts export-fdx PROJECT [PATH]\n\
  sts import PROJECT --script FILE.fountain|.fdx --synopsis FILE [--title TEXT] [--duration N]\n\
  sts import-reviewed-synopsis PROJECT --handoff-root DIR --handoff FILE [--title TEXT] [--duration N]\n\
  sts deliver-context PROJECT\n\
  sts deliver PROJECT REQUEST_JSON\n\
  sts deliver-review PROJECT REQUEST_ID OUTPUT_ID approve|reject [NOTE]\n\
  sts deliver-list PROJECT\n\
  sts deliver-cancel PROJECT\n\n\
Options occur once; -- starts literal positional arguments; --name=VALUE preserves option-looking text.\n\
Existing STS projects can be selected with --project-root DIR; PROJECT must match stored state.\n\
The selected directory is held by the executable; import creation rejects this option.\n\
Assets/projects can be overridden with STS_ASSETS_ROOT and STS_PROJECTS_ROOT.\n\
Selected root paths must be nonempty; relative overrides are anchored to the invocation directory.\n\
Agent timeout can be bounded with STS_CODEX_TIMEOUT_SECS (30..7200; default 1800)."
    );
}
