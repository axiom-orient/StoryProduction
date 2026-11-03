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
    let mut paths = vec![shell_quote(&path)];
    if path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
    {
        paths.push(path);
    }
    for path in paths {
        for reader in ["cat", "/bin/cat", "sed -n '1,240p'"] {
            let body = format!("{reader} {path}");
            for shell in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
                if command == format!("{shell} -lc {}", shell_quote(&body)) {
                    return true;
                }
                // Retain the historical double-quoted event representation only
                // when no expansion, substitution or escape can run inside it.
                if !body.contains(['"', '$', '`', '\\'])
                    && command == format!("{shell} -lc \"{body}\"")
                {
                    return true;
                }
            }
        }
    }
    false
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_cat_and_historical_sed_read_only_the_exact_skill() {
        let path = Path::new("/Users/test/.codex/skills/.system/imagegen/SKILL.md");
        for command in [
            "/bin/zsh -lc 'cat /Users/test/.codex/skills/.system/imagegen/SKILL.md'",
            "/bin/zsh -lc \"sed -n '1,240p' /Users/test/.codex/skills/.system/imagegen/SKILL.md\"",
        ] {
            assert!(is_allowed_skill_read(command, path));
        }
        for command in [
            "/bin/zsh -lc 'cat /Users/test/.codex/auth.json'",
            "/bin/zsh -lc 'cat /Users/test/.codex/skills/.system/imagegen/SKILL.md; touch marker'",
            "/bin/zsh -lc 'cat $(touch marker)/SKILL.md'",
        ] {
            assert!(!is_allowed_skill_read(command, path));
        }
        let injected = Path::new("/tmp/$(touch marker)/SKILL.md");
        assert!(!is_allowed_skill_read(
            "/bin/zsh -lc \"cat /tmp/$(touch marker)/SKILL.md\"",
            injected
        ));
    }
}
