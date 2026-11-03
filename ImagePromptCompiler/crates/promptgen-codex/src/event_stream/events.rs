use std::collections::BTreeMap;

use promptgen_core::json::JsonValue;

use super::as_string;
use super::policy::{ItemKind, classify_item};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CodexEvent {
    ThreadIdentity {
        id: String,
        contract: &'static str,
    },
    TurnCompleted,
    Failure(String),
    ForbiddenAction(String),
    /// An event type or item kind that is not on an explicit allowlist.
    UnknownAction(String),
    /// A known inert top-level event or turn item. It still counts as activity.
    BenignEvent(String),
    /// The one shell read Codex 0.144 performs to load the built-in image skill.
    SkillRead {
        id: String,
        command: String,
        completed: bool,
        succeeded: bool,
    },
}

/// Phase of a single Codex turn.
///
/// The published artifact is trusted only because the turn provably did nothing but
/// generate one image, so "what may still happen" is real state: an identity or an
/// action arriving after `turn.completed` means the log does not describe one closed
/// turn and the artifact cannot be attributed to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EventPhase {
    AwaitingIdentity,
    Running,
    Completed,
}

impl CodexEvent {
    pub(super) fn decode(value: JsonValue) -> Self {
        let JsonValue::Object(object) = value else {
            return Self::Failure("top-level event is not an object".to_owned());
        };
        let event_type = object.get("type").and_then(as_string).unwrap_or_default();
        match event_type {
            "thread.started" => thread_identity(&object, "thread_id", "thread.started/thread_id"),
            "session.created" => {
                thread_identity(&object, "session_id", "session.created/session_id")
            }
            "turn.completed" => Self::TurnCompleted,
            "turn.failed" | "error" => Self::Failure(event_message(&object)),
            "turn.started" => Self::BenignEvent(event_type.to_owned()),
            "item.started" | "item.updated" | "item.completed" => {
                let item = object.get("item").and_then(JsonValue::as_object);
                let kind = item
                    .and_then(|item| {
                        item.get("type")
                            .or_else(|| item.get("kind"))
                            .and_then(as_string)
                    })
                    .unwrap_or_default();
                match classify_item(kind) {
                    ItemKind::Benign => Self::BenignEvent(kind.to_owned()),
                    ItemKind::SkillRead => match item {
                        Some(item) => Self::SkillRead {
                            id: item
                                .get("id")
                                .and_then(as_string)
                                .unwrap_or_default()
                                .to_owned(),
                            command: item
                                .get("command")
                                .and_then(as_string)
                                .unwrap_or_default()
                                .to_owned(),
                            completed: event_type == "item.completed",
                            succeeded: item.get("status").and_then(as_string) == Some("completed")
                                && item.get("exit_code").and_then(JsonValue::as_i64) == Some(0),
                        },
                        None => Self::ForbiddenAction(
                            "command_execution missing item object".to_owned(),
                        ),
                    },
                    ItemKind::Failure => Self::Failure(format!("turn item reported {kind}")),
                    ItemKind::Forbidden => Self::ForbiddenAction(kind.to_owned()),
                    ItemKind::Unknown => Self::UnknownAction(format!("item kind {kind:?}")),
                }
            }
            _ => Self::UnknownAction(format!("top-level event type {event_type:?}")),
        }
    }
}

fn thread_identity(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
    contract: &'static str,
) -> CodexEvent {
    match object
        .get(field)
        .and_then(as_string)
        .filter(|id| !id.is_empty())
    {
        Some(id) => CodexEvent::ThreadIdentity {
            id: id.to_owned(),
            contract,
        },
        None => CodexEvent::Failure(format!("event is missing non-empty {field}")),
    }
}

fn event_message(object: &BTreeMap<String, JsonValue>) -> String {
    object
        .get("message")
        .and_then(as_string)
        .or_else(|| {
            object.get("error").and_then(|value| match value {
                JsonValue::String(value) => Some(value.as_str()),
                JsonValue::Object(error) => error.get("message").and_then(as_string),
                _ => None,
            })
        })
        .unwrap_or("unspecified Codex error")
        .to_owned()
}
