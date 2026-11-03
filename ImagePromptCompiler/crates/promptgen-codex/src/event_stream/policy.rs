use std::path::Path;

pub(super) enum ItemKind {
    Benign,
    SkillRead,
    Failure,
    Forbidden,
    Unknown,
}

/// Classifies a turn item.
///
/// A deny list alone admits every kind that has not been thought of yet: one new
/// tool item type in a future Codex release would pass unnoticed and still publish
/// an artifact. Only the kinds known to be inert are accepted; anything else fails
/// the run and names itself in the error so the list can be revisited deliberately.
pub(super) fn classify_item(kind: &str) -> ItemKind {
    match kind {
        "agent_message"
        | "assistant_message"
        | "user_message"
        | "system_message"
        | "reasoning"
        | "todo_list"
        | "image_generation"
        | "image_generation_call" => ItemKind::Benign,
        // An error item is a reported failure, not inert content.
        "error" => ItemKind::Failure,
        // A command execution is decoded with its identity and command text, then
        // accepted only when it is the exact built-in image-skill read. Read-only
        // sandbox mode is defense in depth, not the authorization decision.
        "command_execution" => ItemKind::SkillRead,
        "file_change" | "mcp_tool_call" | "web_search" | "dynamic_tool_call"
        | "collab_tool_call" => ItemKind::Forbidden,
        _ => ItemKind::Unknown,
    }
}

pub(super) fn is_allowed_skill_read(command: &str, allowed_skill_path: &Path) -> bool {
    let path = allowed_skill_path.display().to_string();
    let unquoted = format!("/bin/zsh -lc \"sed -n '1,240p' {path}\"");
    if command == unquoted {
        return true;
    }
    !path.contains('\'') && command == format!("/bin/zsh -lc \"sed -n '1,240p' '{path}'\"")
}
