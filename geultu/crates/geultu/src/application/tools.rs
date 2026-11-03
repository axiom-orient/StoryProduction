//! Public tool-schema catalog, preserving the established transport order.

mod bootstrap;
mod common;
mod generation;
mod narrative;
mod preflight;
mod project_transfer;
mod refinement;
mod review;
mod visual;
mod voice;

use super::{Application, ToolDefinition};
use crate::error::Result;
use serde_json::Value;
use std::sync::OnceLock;

pub(super) type Handler = fn(&Application, &str, Value) -> Result<Value>;

pub(super) struct RegisteredTool {
    pub(super) definition: ToolDefinition,
    pub(super) handler: Handler,
    pub(super) human_only: bool,
}

/// Static metadata and its owning handler are registered together, in transport order.
pub(super) fn registered_tools() -> &'static [RegisteredTool] {
    static CATALOG: OnceLock<Vec<RegisteredTool>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut tools = Vec::new();
        let mut register = |definitions: Vec<ToolDefinition>, handler: Handler| {
            tools.extend(definitions.into_iter().map(|definition| {
                let human_only = matches!(
                    definition.name,
                    "authoring.review"
                        | "authoring.edit"
                        | "authoring.handoff"
                        | "authoring.restore"
                        | "project.update_metadata"
                        | "candidate.reject"
                        | "changeset.approve"
                        | "changeset.reject"
                        | "synopsis.export_reviewed_handoff"
                );
                RegisteredTool {
                    definition,
                    handler,
                    human_only,
                }
            }));
        };
        register(bootstrap::tools(), super::dispatch::workspace::call);
        register(generation::tools(), super::dispatch::generation::call);
        register(voice::tools(), super::dispatch::voice::call);
        register(project_transfer::tools(), super::dispatch::workspace::call);
        register(narrative::tools(), super::dispatch::narrative::call);
        register(review::tools(), super::dispatch::review::call);
        for (definition, handler) in preflight::tools() {
            register(vec![definition], handler);
        }
        register(refinement::tools(), super::dispatch::refinement::call);
        register(visual::tools(), super::dispatch::visual::call);
        register(crate::authoring::api::tools(), crate::authoring::api::call);
        tools
    })
}

pub(super) fn lookup(name: &str) -> Option<&'static RegisteredTool> {
    registered_tools()
        .iter()
        .find(|tool| tool.definition.name == name)
}
