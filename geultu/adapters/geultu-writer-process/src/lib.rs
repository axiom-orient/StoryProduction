mod process;
mod workspace;

use geultu::generation::{GeneratedDraft, GenerationPlan, GenerationProvider};
use geultu::{Error, Result};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use self::process::run_writer;
use self::workspace::{read_bounded, ProcessFiles};

/// Effect policy, separate from the GenerationPlan and canonical writing state.
#[derive(Debug, Clone)]
pub struct ProcessPolicy {
    pub timeout: Duration,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

impl Default for ProcessPolicy {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(240),
            // JSON escaping may expand each content byte. This is a transport
            // bound, not a replacement for the core's decoded text limit.
            max_stdout_bytes: geultu::model::MAX_TEXT_BYTES * 6 + 64 * 1024,
            max_stderr_bytes: 64 * 1024,
        }
    }
}

impl ProcessPolicy {
    fn validate(&self) -> Result<()> {
        if self.timeout.is_zero()
            || Instant::now().checked_add(self.timeout).is_none()
            || self.max_stdout_bytes == 0
            || self.max_stderr_bytes == 0
            || self.max_stdout_bytes == usize::MAX
            || self.max_stderr_bytes == usize::MAX
        {
            return Err(Error::InvalidInput(
                "writer process policy has an invalid bound".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProcessWriter {
    id: String,
    program: PathBuf,
    args: Vec<String>,
    policy: ProcessPolicy,
}

impl ProcessWriter {
    pub fn new(
        id: impl Into<String>,
        program: impl Into<PathBuf>,
        args: Vec<String>,
    ) -> Result<Self> {
        let id = id.into();
        let program = program.into();
        if id.trim().is_empty() {
            return Err(Error::InvalidInput(
                "writer provider id must not be empty".into(),
            ));
        }
        if program.as_os_str().is_empty() {
            return Err(Error::InvalidInput(
                "writer program must not be empty".into(),
            ));
        }
        Ok(Self {
            id,
            program,
            args,
            policy: ProcessPolicy::default(),
        })
    }

    pub fn with_policy(mut self, policy: ProcessPolicy) -> Result<Self> {
        policy.validate()?;
        self.policy = policy;
        Ok(self)
    }
}

impl GenerationProvider for ProcessWriter {
    fn id(&self) -> &str {
        &self.id
    }

    fn generate(&self, plan: &GenerationPlan) -> Result<GeneratedDraft> {
        self.policy.validate()?;
        let input = serde_json::to_vec(plan)?;
        let workspace = ProcessFiles::create()?;
        let mut stdin = workspace.file("stdin.json")?;
        stdin.write_all(&input)?;
        stdin.seek(SeekFrom::Start(0))?;
        let mut stdout = workspace.file("stdout.json")?;
        let mut stderr = workspace.file("stderr.log")?;
        // Regular private files avoid a full-duplex pipe deadlock and allow the
        // owner to observe time/output bounds even when the child ignores stdin.
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout.try_clone()?))
            .stderr(Stdio::from(stderr.try_clone()?));
        let status = run_writer(&mut command, &stdout, &stderr, &self.policy, &self.id)?;
        if !status.success() {
            let detail = read_bounded(&mut stderr, self.policy.max_stderr_bytes, "stderr")?;
            return Err(Error::AdapterUnavailable(format!(
                "writer provider '{}' failed: {}",
                self.id,
                String::from_utf8_lossy(&detail).trim()
            )));
        }
        let bytes = read_bounded(&mut stdout, self.policy.max_stdout_bytes, "stdout")?;
        serde_json::from_slice(&bytes).map_err(|error| {
            Error::AdapterUnavailable(format!(
                "writer provider '{}' returned invalid JSON: {error}",
                self.id
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geultu::generation::{OutputFormat, WritingStage, WritingType, NOVEL};
    use std::fs;

    #[cfg(unix)]
    #[test]
    fn process_boundary_accepts_structured_draft() {
        let root =
            std::env::temp_dir().join(format!("geultu-writer-process-{}", std::process::id()));
        let _ = fs::remove_file(&root);
        let script = "printf '%s' '{\"content\":\"generated\",\"model\":\"fixture\"}'";
        fs::write(&root, script).unwrap();
        let writer = ProcessWriter::new(
            "fixture",
            "/bin/sh",
            vec![
                "-c".into(),
                format!("cat >/dev/null; {}", fs::read_to_string(&root).unwrap()),
            ],
        )
        .unwrap();
        let draft = writer
            .generate(&GenerationPlan {
                project_id: "book".into(),
                writing_type: WritingType::new(NOVEL).unwrap(),
                stage: WritingStage::new("draft").unwrap(),
                adapter_id: "novel".into(),
                source_content_revision: 0,
                prompt: "prompt".into(),
                output_format: OutputFormat::Markdown,
                input_hash: "hash".into(),
                craftpack: None,
            })
            .unwrap();
        assert_eq!(draft.content, "generated");
        let _ = fs::remove_file(root);
    }
    fn fixture_plan() -> GenerationPlan {
        GenerationPlan {
            project_id: "book".into(),
            writing_type: WritingType::new(NOVEL).unwrap(),
            stage: WritingStage::new("draft").unwrap(),
            adapter_id: "novel".into(),
            source_content_revision: 0,
            prompt: "prompt".repeat(32768),
            output_format: OutputFormat::Markdown,
            input_hash: "hash".into(),
            craftpack: None,
        }
    }

    #[cfg(unix)]
    #[test]
    fn timeout_and_output_limits_are_explicit_failures() {
        let timeout = ProcessWriter::new(
            "fixture",
            "/bin/sh",
            vec!["-c".into(), "exec sleep 5".into()],
        )
        .unwrap()
        .with_policy(ProcessPolicy {
            timeout: Duration::from_millis(50),
            ..ProcessPolicy::default()
        })
        .unwrap();
        assert!(timeout
            .generate(&fixture_plan())
            .unwrap_err()
            .to_string()
            .contains("timeout"));
        let oversized = ProcessWriter::new(
            "fixture",
            "/bin/sh",
            vec!["-c".into(), "printf '%0100d' 1".into()],
        )
        .unwrap()
        .with_policy(ProcessPolicy {
            max_stdout_bytes: 32,
            ..ProcessPolicy::default()
        })
        .unwrap();
        assert!(oversized
            .generate(&fixture_plan())
            .unwrap_err()
            .to_string()
            .contains("byte limit"));
    }

    #[cfg(unix)]
    #[test]
    fn output_before_large_input_cannot_deadlock_pipes() {
        let script = "printf '{\"content\":\"'; head -c 131072 /dev/zero | tr '\\000' x; printf '\",\"model\":\"fixture\"}'; cat >/dev/null";
        let writer =
            ProcessWriter::new("fixture", "/bin/sh", vec!["-c".into(), script.into()]).unwrap();
        assert_eq!(
            writer.generate(&fixture_plan()).unwrap().content.len(),
            131072
        );
    }

    #[test]
    fn zero_policy_is_rejected_before_spawn() {
        assert!(ProcessWriter::new("fixture", "not-executed", vec![])
            .unwrap()
            .with_policy(ProcessPolicy {
                timeout: Duration::ZERO,
                ..ProcessPolicy::default()
            })
            .is_err());
    }
}
