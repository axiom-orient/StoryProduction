use std::path::PathBuf;
use std::time::Duration;

use crate::model::workflow::GenerationPolicy;
use narrative_courier_runtime::{Error, Result};

pub(crate) fn selected_generation_policy() -> Result<GenerationPolicy> {
    let model = configured_value("STS_CODEX_MODEL", "gpt-5.6-sol")?;
    let reasoning_effort = configured_value("STS_CODEX_REASONING", "medium")?;
    let policy = GenerationPolicy {
        model,
        reasoning_effort,
    };
    validate_generation_policy(&policy)?;
    Ok(policy)
}

pub(super) fn codex_timeout() -> Result<Duration> {
    let seconds = match std::env::var("STS_CODEX_TIMEOUT_SECS") {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| Error::Contract("STS_CODEX_TIMEOUT_SECS must be an integer".into()))?,
        Err(std::env::VarError::NotPresent) => 1800,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(Error::Contract(
                "STS_CODEX_TIMEOUT_SECS must be UTF-8".into(),
            ))
        }
    };
    if !(30..=7200).contains(&seconds) {
        return Err(Error::Contract(
            "STS_CODEX_TIMEOUT_SECS must be between 30 and 7200".into(),
        ));
    }
    Ok(Duration::from_secs(seconds))
}

pub(super) fn resolve_codex() -> Result<PathBuf> {
    if let Some(value) = std::env::var_os("STS_CODEX_BIN") {
        if value.is_empty() {
            return Err(Error::Contract("STS_CODEX_BIN cannot be empty".into()));
        }
        return Ok(PathBuf::from(value));
    }
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local/bin/codex"));
        candidates.push(home.join(".npm-global/bin/codex"));
    }
    Ok(candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("codex")))
}

pub(crate) fn validate_generation_policy(policy: &GenerationPolicy) -> Result<()> {
    if policy.model.trim().is_empty() {
        return Err(Error::Contract("Codex model cannot be empty".into()));
    }
    if !matches!(
        policy.reasoning_effort.as_str(),
        "minimal" | "low" | "medium" | "high" | "xhigh"
    ) {
        return Err(Error::Contract(
            "Codex reasoning must be minimal|low|medium|high|xhigh".into(),
        ));
    }
    Ok(())
}

fn configured_value(name: &str, default: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(default.into()),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(Error::Contract(format!("{name} must be UTF-8")))
        }
    }
}
