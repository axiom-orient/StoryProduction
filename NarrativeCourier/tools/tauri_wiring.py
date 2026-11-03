#!/usr/bin/env python3
"""Read-only lexical checks for the Tauri UI command boundary."""

from __future__ import annotations

from pathlib import Path
import re


def inspect_tauri_wiring(root: Path, ui_source: str) -> list[tuple[bool, str]]:
    """Relate UI invokes, the app's handler allowlist, and command definitions."""
    entry_path = root / "src-tauri/src/lib.rs"
    entry_source = entry_path.read_text(encoding="utf-8")
    source_paths = sorted((root / "src-tauri/src").rglob("*.rs"))
    command_source = "\n".join(path.read_text(encoding="utf-8") for path in source_paths)

    commands = set(
        re.findall(
            r"#\[tauri::command\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)",
            command_source,
        )
    )
    invokes = set(re.findall(r'invoke\("([a-zA-Z0-9_]+)"', ui_source))
    handler_match = re.search(r"tauri::generate_handler!\s*\[(.*?)\]", entry_source, re.S)
    registration_paths = (
        [path.strip() for path in handler_match.group(1).split(",") if path.strip()]
        if handler_match
        else []
    )
    registered: set[str] = set()
    malformed_registrations = []
    for path in registration_paths:
        match = re.fullmatch(r"(?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Za-z_][A-Za-z0-9_]*)", path)
        if match:
            registered.add(match.group(1))
        else:
            malformed_registrations.append(path)

    if (
        handler_match
        and not malformed_registrations
        and invokes <= registered
        and registered == commands
        and len(registration_paths) == len(registered)
    ):
        command_result = (
            True,
            f"UI/backend wiring ({len(invokes)} invokes, {len(registered)} registered commands)",
        )
    else:
        missing = sorted(invokes - registered)
        unregistered = sorted(commands - registered)
        undefined = sorted(registered - commands)
        command_result = (
            False,
            "Tauri command wiring mismatch: "
            f"UI missing registration={missing}, "
            f"definitions without registration={unregistered}, "
            f"registered without definition={undefined}, "
            f"malformed registrations={malformed_registrations}",
        )

    decision_signature = re.search(
        r"#\[tauri::command\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?"
        r"fn\s+decide_review\s*\((.*?)\)",
        command_source,
        re.S,
    )
    token_wired = bool(
        decision_signature
        and "gate_token: String" in decision_signature.group(1)
        and re.search(
            r'invoke\("decide_review",\s*\{[^}]*gateToken\s*:\s*[A-Za-z_$][\w$]*\.gate_token',
            ui_source,
        )
    )
    token_result = (
        (True, "Human causal token UI→backend")
        if token_wired
        else (False, "Human gate token wiring")
    )
    return [command_result, token_result]
