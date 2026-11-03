use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::codex::config::{codex_timeout, resolve_codex, validate_generation_policy};
use crate::model::workflow::GenerationPolicy;
use narrative_courier_runtime::process::run_piped_command;
use narrative_courier_runtime::{Error, Result};

pub(super) fn run_codex(
    task: &str,
    binary_inputs: &[PathBuf],
    policy: &GenerationPolicy,
    cancel_path: &Path,
    run_id: &str,
) -> Result<String> {
    validate_generation_policy(policy)?;
    let timeout = codex_timeout()?;
    let codex = resolve_codex()?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Contract("system clock is before the Unix epoch".into()))?
        .as_nanos();
    let workdir = std::env::temp_dir().join(format!(
        "narrative-courier-codex-{}-{stamp}",
        std::process::id()
    ));

    create_private_workdir(&workdir)?;
    let result = (|| -> Result<String> {
        let mut prompt = task.to_string();
        let mut images = Vec::new();
        if !binary_inputs.is_empty() {
            let input_dir = workdir.join("inputs");
            std::fs::create_dir_all(&input_dir)?;
            for (index, source) in binary_inputs.iter().enumerate() {
                let name = source
                    .file_name()
                    .and_then(|v| v.to_str())
                    .ok_or_else(|| Error::Contract("binary input filename must be UTF-8".into()))?;
                let target = input_dir.join(format!("{index:02}-{name}"));
                std::fs::copy(source, &target)?;
                prompt =
                    prompt.replace(&source.display().to_string(), &target.display().to_string());
                if is_image(&target) {
                    images.push(target);
                }
            }
        }

        let last_message = workdir.join("artifact.txt");
        let mut command = Command::new(&codex);
        command
            .arg("--strict-config")
            .arg("--ask-for-approval")
            .arg("never")
            .arg("exec")
            .arg("--ignore-rules")
            .arg("--ignore-user-config")
            .arg("--model")
            .arg(&policy.model)
            .arg("--config")
            .arg(format!(
                "model_reasoning_effort=\"{}\"",
                policy.reasoning_effort
            ))
            .arg("--config")
            .arg("web_search=\"disabled\"")
            .arg("--ephemeral")
            .arg("--sandbox")
            .arg("read-only")
            .arg("--skip-git-repo-check")
            .arg("--output-last-message")
            .arg(&last_message);
        for image in &images {
            command.arg("--image").arg(image);
        }
        command
            .arg("-")
            .current_dir(&workdir)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());

        let status = run_piped_command(
            &mut command,
            prompt.as_bytes(),
            timeout,
            cancel_path,
            run_id,
        )
        .map_err(|e| {
            Error::Executor(format!(
                "Codex execution failed at {}: {e}",
                codex.display()
            ))
        })?;
        if !status.success() {
            return Err(Error::Executor(format!(
                "Codex exited with status {status}"
            )));
        }
        let metadata = std::fs::symlink_metadata(&last_message).map_err(|error| {
            Error::Executor(format!("Codex final artifact was not written: {error}"))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Error::Executor(
                "Codex final artifact must be a regular file".into(),
            ));
        }
        let text = std::fs::read_to_string(&last_message)
            .map_err(|e| Error::Executor(format!("Codex final artifact was not written: {e}")))?;
        if text.trim().is_empty() {
            return Err(Error::Executor("Codex returned an empty artifact".into()));
        }
        Ok(text)
    })();

    match std::fs::remove_dir_all(&workdir) {
        Ok(()) => result,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => result,
        Err(cleanup_error) => match result {
            Ok(_) => Err(Error::Executor(format!(
                "Codex workspace cleanup failed: {cleanup_error}"
            ))),
            Err(run_error) => Err(Error::Executor(format!(
                "{run_error}; Codex workspace cleanup also failed: {cleanup_error}"
            ))),
        },
    }
}

fn create_private_workdir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700).create(path)?;
    }
    #[cfg(not(unix))]
    std::fs::create_dir(path)?;
    Ok(())
}

fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "gif")
    )
}
