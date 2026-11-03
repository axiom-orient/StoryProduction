use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::codex::config::{resolve_codex, selected_generation_policy};
use crate::model::views::RuntimeStatus;
use crate::Engine;
use narrative_courier_runtime::process::run_piped_command_capture;

impl Engine {
    pub fn codex_runtime_status(&self) -> RuntimeStatus {
        let policy = selected_generation_policy();
        let (model, reasoning_effort, policy_error) = match policy {
            Ok(policy) => (policy.model, policy.reasoning_effort, None),
            Err(error) => (String::new(), String::new(), Some(error.to_string())),
        };
        let probe = resolve_codex().and_then(|codex| {
            let output = codex_probe(&codex, &["--version"])?;
            if !output.status.success() {
                return Err(narrative_courier_runtime::Error::Executor(format!(
                    "Codex version probe exited with {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            let version = String::from_utf8(output.stdout).map_err(|error| {
                narrative_courier_runtime::Error::Executor(format!(
                    "Codex version is not UTF-8: {error}"
                ))
            })?;
            let version = version.trim().to_owned();
            if version.is_empty() {
                return Err(narrative_courier_runtime::Error::Executor(
                    "Codex version probe returned no version".into(),
                ));
            }
            Ok((codex, version))
        });
        let (codex_found, version, authenticated, probe_error) = match probe {
            Ok((codex, version)) => match codex_probe(&codex, &["login", "status"]) {
                Ok(output) => (true, Some(version), output.status.success(), None),
                Err(error) => (true, Some(version), false, Some(error.to_string())),
            },
            Err(error) => (false, None, false, Some(error.to_string())),
        };
        let message = if let Some(error) = &policy_error {
            format!("Codex generation policy is invalid: {error}")
        } else if let Some(error) = probe_error {
            format!("Codex runtime probe failed: {error}")
        } else if authenticated {
            "Codex is ready.".into()
        } else {
            "Codex is installed but not authenticated. Run `codex login`.".into()
        };
        RuntimeStatus {
            ready: codex_found && authenticated && policy_error.is_none(),
            codex_found,
            authenticated,
            version,
            model,
            reasoning_effort,
            message,
        }
    }
}

fn codex_probe(
    codex: &Path,
    args: &[&str],
) -> narrative_courier_runtime::Result<std::process::Output> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            narrative_courier_runtime::Error::Contract(
                "system clock is before the Unix epoch".into(),
            )
        })?
        .as_nanos();
    let run_id = format!("codex-probe-{}-{stamp}", std::process::id());
    let cancel_path = std::env::temp_dir().join(format!(".{run_id}.cancel"));
    let mut command = Command::new(codex);
    command.args(args);
    run_piped_command_capture(
        &mut command,
        b"",
        Duration::from_secs(5),
        &cancel_path,
        &run_id,
    )
}
