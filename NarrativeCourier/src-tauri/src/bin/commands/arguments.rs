//! Closed CLI argument admission before application configuration or domain effects.

use super::CliResult;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(crate) enum Command {
    Artifact,
    SourceBinding,
    Run,
    Cancel,
    Next,
    AgentTask,
    Submit,
    Decide,
    Doctor,
    Status,
    Resume,
    ExportFountain,
    ExportFdx,
    Import,
    ImportReviewedSynopsis,
    Export,
    DeliverContext,
    Deliver,
    DeliverReview,
    DeliverList,
    DeliverCancel,
}

enum Value {
    Text(String),
    Step(u8),
    Duration(u32),
}

pub(crate) struct Invocation {
    pub command: Command,
    pub project: String,
    pub positional: Vec<String>,
    options: BTreeMap<String, Value>,
}

impl Invocation {
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.options.get(key) {
            Some(Value::Text(value)) => Some(value),
            _ => None,
        }
    }
    pub fn required_text(&self, key: &str) -> CliResult<&str> {
        self.text(key)
            .ok_or_else(|| format!("required admitted option missing: {key}").into())
    }
    pub fn step(&self) -> Option<u8> {
        match self.options.get("--step") {
            Some(Value::Step(value)) => Some(*value),
            _ => None,
        }
    }
    pub fn duration(&self) -> u32 {
        match self.options.get("--duration") {
            Some(Value::Duration(value)) => *value,
            _ => 100,
        }
    }
}

pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> CliResult<Option<Invocation>> {
    let mut args = args.into_iter();
    let name = args.next().unwrap_or_else(|| "help".into());
    if matches!(name.as_str(), "help" | "--help" | "-h") {
        if args.next().is_some() {
            return Err("help takes no arguments".into());
        }
        return Ok(None);
    }
    let (command, minimum, maximum, allowed, required): (Command, usize, usize, &[&str], &[&str]) =
        match name.as_str() {
            "artifact" => (Command::Artifact, 2, 2, &[], &[]),
            "source-binding" => (Command::SourceBinding, 1, 1, &[], &[]),
            "run" => (Command::Run, 1, 1, &[], &[]),
            "cancel" => (Command::Cancel, 1, 1, &[], &[]),
            "next" => (Command::Next, 1, 1, &[], &[]),
            "agent-task" => (Command::AgentTask, 1, 1, &[], &[]),
            "submit" => (Command::Submit, 2, 2, &["--task-token"], &["--task-token"]),
            "decide" => (
                Command::Decide,
                2,
                2,
                &["--step", "--feedback", "--gate-token"],
                &["--gate-token"],
            ),
            "doctor" => (Command::Doctor, 1, 1, &[], &[]),
            "status" => (Command::Status, 1, 1, &[], &[]),
            "resume" => (Command::Resume, 1, 1, &[], &[]),
            "export-fountain" => (Command::ExportFountain, 1, 2, &[], &[]),
            "export-fdx" => (Command::ExportFdx, 1, 2, &[], &[]),
            "export" => (Command::Export, 1, 2, &[], &[]),
            "import" => (
                Command::Import,
                1,
                1,
                &["--script", "--synopsis", "--title", "--duration"],
                &["--script", "--synopsis"],
            ),
            "import-reviewed-synopsis" => (
                Command::ImportReviewedSynopsis,
                1,
                1,
                &["--handoff-root", "--handoff", "--title", "--duration"],
                &["--handoff"],
            ),
            "deliver-context" => (Command::DeliverContext, 1, 1, &[], &[]),
            "deliver" => (Command::Deliver, 2, 2, &[], &[]),
            "deliver-review" => (Command::DeliverReview, 4, 5, &[], &[]),
            "deliver-list" => (Command::DeliverList, 1, 1, &[], &[]),
            "deliver-cancel" => (Command::DeliverCancel, 1, 1, &[], &[]),
            _ => return Err(format!("unknown command: {name}; use --help").into()),
        };
    let mut positional = Vec::new();
    let mut options = BTreeMap::new();
    let mut literal = false;
    while let Some(argument) = args.next() {
        if !literal && argument == "--" {
            literal = true;
            continue;
        }
        if !literal && argument.starts_with("--") {
            let (key, inline) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(key, value)| (key, Some(value)));
            if !allowed.contains(&key) && key != "--project-root" {
                return Err(format!("unknown {name} option: {key}").into());
            }
            if options.contains_key(key) {
                return Err(format!("duplicate option: {key}").into());
            }
            let value = match inline {
                Some(value) => value.to_owned(),
                None => {
                    let value = args.next().ok_or_else(|| format!("{key} requires VALUE"))?;
                    if value == "--" || value.starts_with("--") {
                        return Err(format!(
                            "{key} requires VALUE; use {key}=VALUE for text starting with --"
                        )
                        .into());
                    }
                    value
                }
            };
            if value.is_empty() && key != "--feedback" {
                return Err(format!("{key} requires a nonempty value").into());
            }
            let parsed = match key {
                "--step" => Value::Step(
                    value
                        .parse::<u8>()
                        .map_err(|error| format!("{key} invalid value: {error}"))?,
                ),
                "--duration" => {
                    let duration = value
                        .parse::<u32>()
                        .map_err(|error| format!("{key} invalid value: {error}"))?;
                    if duration == 0 {
                        return Err("--duration must be positive".into());
                    }
                    Value::Duration(duration)
                }
                _ => Value::Text(value),
            };
            options.insert(key.to_owned(), parsed);
        } else {
            positional.push(argument);
        }
    }
    if positional.len() < minimum || positional.len() > maximum {
        return Err(format!(
            "{name} requires {minimum}..{maximum} positional arguments including PROJECT"
        )
        .into());
    }
    for key in required {
        if !options.contains_key(*key) {
            return Err(format!("{key} is required").into());
        }
    }
    if positional
        .iter()
        .take(minimum)
        .any(|value| value.is_empty())
    {
        return Err(format!("{name} required positional values must not be empty").into());
    }
    if options.contains_key("--project-root")
        && matches!(command, Command::Import | Command::ImportReviewedSynopsis)
    {
        return Err("--project-root selects an existing project and cannot be combined with import creation".into());
    }
    let project = positional.remove(0);
    if project.trim().is_empty() {
        return Err("PROJECT is required".into());
    }
    if matches!(command, Command::Decide)
        && !matches!(positional[0].as_str(), "approve" | "revise" | "pause")
    {
        return Err("DECISION must be approve|revise|pause".into());
    }
    if matches!(command, Command::DeliverReview)
        && !matches!(positional[2].as_str(), "approve" | "reject")
    {
        return Err("delivery review decision must be approve|reject".into());
    }
    Ok(Some(Invocation {
        command,
        project,
        positional,
        options,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn invocation(args: &[&str]) -> CliResult<Option<Invocation>> {
        parse(args.iter().map(|value| value.to_string()))
    }
    #[test]
    fn existing_project_root_is_explicit_unique_and_preserves_logical_identity() {
        for args in [
            vec!["agent-task", "demo", "--project-root", "."],
            vec!["agent-task", "--project-root=.", "demo"],
        ] {
            let parsed = invocation(&args).unwrap().unwrap();
            assert_eq!(parsed.project, "demo");
            assert_eq!(parsed.text("--project-root"), Some("."));
        }
        for args in [
            vec!["agent-task", "demo", "--project-root"],
            vec![
                "agent-task",
                "demo",
                "--project-root=.",
                "--project-root=other",
            ],
            vec![
                "import-reviewed-synopsis",
                "demo",
                "--handoff=receipt.json",
                "--project-root=.",
            ],
        ] {
            assert!(invocation(&args).is_err());
        }
    }
}
