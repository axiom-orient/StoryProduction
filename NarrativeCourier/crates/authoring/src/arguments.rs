//! Closed owner CLI grammar. Repeated scenes are explicit; other options occur once.

use crate::{require, Error, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(crate) struct Invocation {
    pub command: String,
    pub root: PathBuf,
    options: BTreeMap<String, String>,
    pub scenes: Vec<String>,
    pub candidate: bool,
}
impl Invocation {
    pub fn text(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(String::as_str)
    }
    pub fn required(&self, key: &str) -> &str {
        self.text(key).expect("closed CLI admitted required option")
    }
}

pub(crate) fn help(lane: &str) -> String {
    let commands = if lane == "storyboard" {
        "  prepare --task-id ID --brief TEXT --output storyboards/TASK.json [--base BOARD] [--scene SCN-ID ...]\n\
  task --task TASK\n\
  submit --task TASK --draft DRAFT --output storyboards/BOARD.json\n\
  review --board BOARD --expected-hash HASH --reviewer NAME --decision approved|rejected --output storyboards/REVIEW.json [--note TEXT]\n\
  export --board BOARD --output storyboards/BOARD.md (--review REVIEW | --candidate)\n\
  cancel --task TASK\n\
  scenes\n\
  bundle --board BOARD --review REVIEW"
    } else {
        "All commands require --board BOARD --board-review REVIEW.\n\
  source\n\
  submit --draft DRAFT --output animatic/PLAN.json\n\
  review --plan PLAN --expected-hash HASH --reviewer NAME --decision approved|rejected --output animatic/REVIEW.json [--note TEXT]\n\
  export --plan PLAN --review REVIEW [--output animatic/PACKET.json]"
    };
    format!("{lane} OP --project-root DIR\n\n{commands}\n\nPaths are project-relative. Options occur once; --scene may repeat for prepare.\n--name=VALUE admits option-looking text. Candidate, human review, and export are separate.")
}

pub(crate) fn parse(lane: &str, args: impl IntoIterator<Item = String>) -> Result<Invocation> {
    let mut args = args.into_iter();
    let command = args
        .next()
        .ok_or_else(|| Error::refusal("MISSING_COMMAND_ARGUMENT"))?;
    let (allowed, required): (&[&str], &[&str]) = match (lane, command.as_str()) {
        ("storyboard", "prepare") => (
            &["task-id", "brief", "base", "scene", "output"],
            &["task-id", "brief", "output"],
        ),
        ("storyboard", "task" | "cancel") => (&["task"], &["task"]),
        ("storyboard", "submit") => (&["task", "draft", "output"], &["task", "draft", "output"]),
        ("storyboard", "review") => (
            &[
                "board",
                "reviewer",
                "output",
                "decision",
                "expected-hash",
                "note",
            ],
            &["board", "reviewer", "output", "decision", "expected-hash"],
        ),
        ("storyboard", "export") => (
            &["board", "output", "review", "candidate"],
            &["board", "output"],
        ),
        ("storyboard", "scenes") => (&[], &[]),
        ("storyboard", "bundle") => (&["board", "review"], &["board", "review"]),
        ("animatic", "source") => (&["board", "board-review"], &["board", "board-review"]),
        ("animatic", "submit") => (
            &["board", "board-review", "draft", "output"],
            &["board", "board-review", "draft", "output"],
        ),
        ("animatic", "review") => (
            &[
                "board",
                "board-review",
                "plan",
                "output",
                "expected-hash",
                "reviewer",
                "decision",
                "note",
            ],
            &[
                "board",
                "board-review",
                "plan",
                "output",
                "expected-hash",
                "reviewer",
                "decision",
            ],
        ),
        ("animatic", "export") => (
            &["board", "board-review", "plan", "review", "output"],
            &["board", "board-review", "plan", "review"],
        ),
        _ => return Err(Error::refusal(format!("UNKNOWN_COMMAND:{lane}:{command}"))),
    };
    let mut options = BTreeMap::new();
    let mut scenes = Vec::new();
    let mut candidate = false;
    while let Some(argument) = args.next() {
        let option = argument
            .strip_prefix("--")
            .ok_or_else(|| Error::refusal(format!("ARGUMENTS:{argument}")))?;
        let (key, inline) = option
            .split_once('=')
            .map_or((option, None), |(key, value)| (key, Some(value)));
        require(
            key == "project-root" || allowed.contains(&key),
            format!("UNSUPPORTED_COMMAND_OPTIONS:{key}"),
        )?;
        if key == "candidate" {
            require(
                !candidate && inline.is_none(),
                "DUPLICATE_OR_VALUE_CANDIDATE_OPTION",
            )?;
            candidate = true;
            continue;
        }
        require(
            key == "scene" || !options.contains_key(key),
            format!("DUPLICATE_OPTION:{key}"),
        )?;
        let value = match inline {
            Some(value) => value.to_owned(),
            None => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::refusal(format!("MISSING_OPTION_VALUE:{key}")))?;
                require(
                    !value.starts_with("--"),
                    format!("MISSING_OPTION_VALUE:{key}"),
                )?;
                value
            }
        };
        if key == "scene" {
            scenes.push(value);
        } else {
            options.insert(key.to_owned(), value);
        }
    }
    for key in required
        .iter()
        .copied()
        .chain(std::iter::once("project-root"))
    {
        require(
            options.get(key).is_some_and(|value| !value.is_empty()),
            format!("MISSING_COMMAND_ARGUMENT:{key}"),
        )?;
    }
    if let Some(decision) = options.get("decision") {
        require(
            matches!(decision.as_str(), "approved" | "rejected"),
            "INVALID_REVIEW_DECISION",
        )?;
    }
    if lane == "storyboard" && command == "export" {
        require(
            !(candidate && options.contains_key("review")),
            "CHOOSE_CANDIDATE_OR_APPROVED_EXPORT",
        )?;
        require(
            candidate || options.get("review").is_some_and(|s| !s.is_empty()),
            "APPROVED_EXPORT_REQUIRES_REVIEW",
        )?;
    }
    let root = PathBuf::from(options.remove("project-root").expect("root admitted"));
    Ok(Invocation {
        command,
        root,
        options,
        scenes,
        candidate,
    })
}
