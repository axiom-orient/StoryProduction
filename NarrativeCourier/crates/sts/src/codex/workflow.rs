mod lease;

use super::runner::run_codex;
use crate::model::views::{NextPacket, RunResult};
use crate::Engine;
use lease::{
    begin_agent_run, cancellation_requested, ensure_active_identity, finish_agent_run,
    read_active_identity, remove_if_exists, request_cancel, AgentRunIdentity,
};
use narrative_courier_runtime::store::{agent_run_is_active, lock_project, try_lock_agent_run};
use narrative_courier_runtime::{Error, Result};

impl Engine {
    /// Execute at most one Agent obligation. Human/pause/completed boundaries are returned without side effects.
    /// This is the UI primitive: it provides observable progress without introducing a second orchestration layer.
    pub fn run_one_codex(&self, name: &str) -> Result<RunResult> {
        let project = self.project_dir(name)?;
        let _run_lock = try_lock_agent_run(&project)?;
        let start = {
            let _workflow_lock = lock_project(&project)?;
            match self.next(name)? {
                NextPacket::AgentTask {
                    node, task_token, ..
                } => {
                    let (material_token, task, binary_inputs, policy) = self.task_material(name)?;
                    if material_token != task_token {
                        return Err(Error::Contract(
                            "Agent task changed while its run was being prepared".into(),
                        ));
                    }
                    let identity = AgentRunIdentity::new(node, task_token.clone())?;
                    begin_agent_run(&project, &identity)?;
                    Start::Agent {
                        identity,
                        task,
                        binary_inputs,
                        policy,
                    }
                }
                NextPacket::AwaitingHuman { node, .. } => Start::Boundary(format!("human:{node}")),
                NextPacket::Paused => Start::Boundary("paused".into()),
                NextPacket::Completed { .. } => Start::Boundary("completed".into()),
            }
        };

        match start {
            Start::Agent {
                identity,
                task,
                binary_inputs,
                policy,
            } => {
                let cancel_path = project.join(".agent-cancel");
                let execution = run_codex(
                    &task,
                    &binary_inputs,
                    &policy,
                    &cancel_path,
                    &identity.run_id,
                );
                let _workflow_lock = lock_project(&project)?;
                ensure_active_identity(&project, &identity)?;
                let cancelled = cancellation_requested(&project, &identity)?;
                let outcome = if cancelled {
                    Err(Error::Executor("Codex execution cancelled".into()))
                } else {
                    execution
                        .and_then(|artifact| {
                            self.submit_with_lock(
                                name,
                                &artifact,
                                &identity.task_token,
                                &_workflow_lock,
                            )
                        })
                        .map(|_| ())
                };
                let cleanup = finish_agent_run(&project, &identity, cancelled);
                match (outcome, cleanup) {
                    (Ok(()), Ok(())) => {}
                    (Err(error), Ok(())) => return Err(error),
                    (Ok(()), Err(cleanup)) => {
                        return Err(Error::Executor(format!(
                            "Agent artifact committed but lease cleanup failed: {cleanup}"
                        )))
                    }
                    (Err(error), Err(cleanup)) => {
                        return Err(Error::Executor(format!(
                            "{error}; Agent lease cleanup also failed: {cleanup}"
                        )))
                    }
                }
                Ok(RunResult {
                    stopped_on: format!("agent:{}", identity.node),
                    tasks_completed: vec![identity.node],
                    project: self.project_view(name)?,
                })
            }
            Start::Boundary(stopped_on) => Ok(RunResult {
                stopped_on,
                tasks_completed: vec![],
                project: self.project_view(name)?,
            }),
        }
    }

    pub fn cancel_codex(&self, name: &str) -> Result<()> {
        let project = self.project_dir(name)?;
        match std::fs::symlink_metadata(&project) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(Error::Contract(
                    "Agent project path is not a regular directory".into(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Contract(
                    "no Agent run is active for this project".into(),
                ));
            }
            Err(error) => return Err(Error::Io(error)),
        }
        let _workflow_lock = lock_project(&project)?;
        if !agent_run_is_active(&project)? {
            remove_if_exists(&project.join(".agent-active"))?;
            return Err(Error::Contract(
                "no Agent run is active for this project".into(),
            ));
        }
        let identity = read_active_identity(&project)?;
        match self.next(name)? {
            NextPacket::AgentTask {
                node, task_token, ..
            } if node == identity.node && task_token == identity.task_token => {}
            _ => {
                return Err(Error::Contract(
                    "Agent run is finishing; cancellation arrived after its task changed".into(),
                ));
            }
        }
        request_cancel(&project, &identity)?;
        Ok(())
    }

    /// Run consecutive semantic obligations with Codex until Human input, pause, or completion.
    /// CLI keeps the one-command model; the desktop UI calls `run_one_codex` repeatedly to expose progress.
    pub fn run_to_human_codex(&self, name: &str) -> Result<RunResult> {
        let mut completed_nodes = Vec::new();
        for _ in 0..32 {
            let result = self.run_one_codex(name)?;
            completed_nodes.extend(result.tasks_completed);
            if !result.stopped_on.starts_with("agent:") {
                return Ok(RunResult {
                    stopped_on: result.stopped_on,
                    tasks_completed: completed_nodes,
                    project: result.project,
                });
            }
        }
        Err(Error::Contract(
            "agent run exceeded 32 obligations without a human boundary".into(),
        ))
    }
}

enum Start {
    Agent {
        identity: AgentRunIdentity,
        task: String,
        binary_inputs: Vec<std::path::PathBuf>,
        policy: crate::model::workflow::GenerationPolicy,
    },
    Boundary(String),
}
