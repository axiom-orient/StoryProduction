#![forbid(unsafe_code)]

mod tool_contract;
mod transport;

pub use transport::{
    DEFAULT_MAX_BODY_BYTES, DEFAULT_MCP_PORT, MAX_BODY_BYTES, MCP_PROTOCOL_VERSION, McpHttpConfig,
    build_router, serve_http, serve_stdio,
};

use std::borrow::Cow;

#[cfg(test)]
use std::io;

use promptgen_core::catalog::{CatalogFamily, catalog_json};
use promptgen_core::image::available_lut_presets;
use promptgen_core::json::{JsonValue, parse};
use promptgen_core::{
    CompilationStatus, ImagePromptRequest, InterviewRequest, InterviewStatus, compile_image_prompt,
    run_interview,
};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorCode, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ResultType, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use serde_json::Value;

use tool_contract::tool_definitions;

#[derive(Clone, Copy, Debug, Default)]
pub struct PromptGenServer;

impl ServerHandler for PromptGenServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2026_07_28)
            .with_server_info(Implementation::new(
                "promptgen-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Image-only, stateless prompt compiler. Discover the server, then use promptgen_interview when an image brief is not yet a complete typed request: call it with {kind:\"image\", brief:<brief>, answers:{}}; when structuredContent.status is needs_input, ask each structuredContent.questions entry and call again with the same brief and accumulated answers keyed by question id. When status is ready, use structuredContent.compilation.prompt as the final prompt (also mirrored in content[0].text); invalid is fail-closed. Modern Streamable HTTP non-initialize requests carry protocol 2026-07-28 metadata and headers; stdio uses newline-delimited JSON and has no HTTP headers.",
            )
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2026_07_28])
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let arguments = request.arguments.unwrap_or_default();
        match request.name.as_ref() {
            "promptgen_compile_image" => {
                let value = decode_arguments(arguments)?;
                let request = ImagePromptRequest::from_json(value)
                    .map_err(|error| tool_input_error(error.to_string()))?;
                Ok(compilation_response(compile_image_prompt(&request)))
            }
            "promptgen_interview" => {
                let value = decode_arguments(arguments)?;
                let request = InterviewRequest::from_json(value)
                    .map_err(|error| tool_input_error(error.to_string()))?;
                let outcome = run_interview(&request);
                let text = interview_to_text(&outcome);
                let is_error = outcome.status == InterviewStatus::Invalid;
                Ok(json_response(outcome.to_json(), text, is_error))
            }
            "promptgen_catalog" => {
                let family = parse_catalog_family(&arguments)?;
                let value =
                    catalog_json(family).map_err(|error| tool_internal_error(error.to_string()))?;
                let text = value.to_pretty_string();
                Ok(json_response(value, text, false))
            }
            "promptgen_lut_presets" => {
                reject_extra_arguments(&arguments)?;
                let value = available_lut_presets();
                let text = value.to_pretty_string();
                Ok(json_response(value, text, false))
            }
            name => Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("unknown promptGen MCP tool: {name}"),
                None,
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tool_definitions()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_definitions()
            .into_iter()
            .find(|tool| tool.name == name)
    }
}

fn decode_arguments(arguments: JsonObject) -> Result<JsonValue, ErrorData> {
    let value = Value::Object(arguments);
    parse(&value.to_string()).map_err(|error| tool_input_error(error.to_string()))
}

fn reject_extra_arguments(arguments: &JsonObject) -> Result<(), ErrorData> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(tool_input_error(format!(
            "unexpected arguments: {}",
            arguments.keys().cloned().collect::<Vec<_>>().join(", ")
        )))
    }
}

fn parse_catalog_family(arguments: &JsonObject) -> Result<Option<CatalogFamily>, ErrorData> {
    let Some(family) = arguments.get("family") else {
        reject_extra_arguments(arguments)?;
        return Ok(None);
    };
    if arguments.len() != 1 {
        return Err(tool_input_error(
            "promptgen_catalog accepts only the family argument",
        ));
    }
    let Some(family) = family.as_str() else {
        return Err(tool_input_error("catalog family must be a string"));
    };
    if family == "all" {
        Ok(None)
    } else {
        CatalogFamily::parse(family)
            .map(Some)
            .ok_or_else(|| tool_input_error(format!("unknown catalog family: {family}")))
    }
}

fn compilation_response(outcome: promptgen_core::CompilationOutcome) -> CallToolResponse {
    let is_error = outcome.status == CompilationStatus::Invalid;
    json_response(outcome.to_json(), outcome.to_text(), is_error)
}

fn interview_to_text(outcome: &promptgen_core::InterviewOutcome) -> String {
    if let Some(compilation) = &outcome.compilation {
        return compilation.to_text();
    }
    let mut text = format!(
        "status={}\nkind={}\n",
        outcome.status.as_str(),
        outcome.kind.as_str()
    );
    if !outcome.inferences.is_empty() {
        text.push_str("\n[inferences]\n");
        for inference in &outcome.inferences {
            text.push_str("- ");
            text.push_str(inference);
            text.push('\n');
        }
    }
    if !outcome.questions.is_empty() {
        text.push_str("\n[questions]\n");
        for question in &outcome.questions {
            text.push_str(&format!(
                "- {}: {} ({})\n  {}\n",
                question.id, question.label, question.control, question.help
            ));
        }
    }
    text
}

fn tool_input_error(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

fn tool_internal_error(message: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(message.into(), None)
}

fn json_response(value: JsonValue, text: String, is_error: bool) -> CallToolResponse {
    let value = serde_json::from_str(&value.to_compact_string())
        .expect("promptGen output must be valid JSON");
    let mut result = if is_error {
        CallToolResult::structured_error(value)
    } else {
        CallToolResult::structured(value)
    };
    result.content = vec![ContentBlock::text(text)];
    result.result_type = Some(ResultType::COMPLETE);
    result.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_stateless_protocol_is_advertised() {
        assert_eq!(
            PromptGenServer.supported_protocol_versions().as_ref(),
            &[ProtocolVersion::V_2026_07_28]
        );
        assert_eq!(
            PromptGenServer.get_info().protocol_version,
            ProtocolVersion::V_2026_07_28
        );
    }

    #[test]
    fn instructions_explain_the_stateless_interview_output_contract() {
        let instructions = PromptGenServer
            .get_info()
            .instructions
            .expect("server instructions");
        for required in [
            "promptgen_interview",
            "structuredContent.questions",
            "structuredContent.compilation.prompt",
            "invalid is fail-closed",
            "stdio uses newline-delimited JSON",
        ] {
            assert!(
                instructions.contains(required),
                "server instructions omit {required:?}: {instructions}"
            );
        }
    }

    #[test]
    fn remote_bind_requires_authentication_and_host_allowlist() {
        let config = McpHttpConfig {
            bind: "0.0.0.0:4174".parse().expect("socket address"),
            ..McpHttpConfig::default()
        };
        assert_eq!(
            config
                .validate()
                .expect_err("remote bind must be protected")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn tool_definitions_use_canonical_schemas() {
        let tools = tool_definitions();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().all(|tool| {
            tool.annotations
                .as_ref()
                .is_some_and(|annotations| annotations.is_idempotent())
        }));
        assert!(
            tools
                .iter()
                .any(|tool| tool.name == "promptgen_compile_image")
        );
    }
}
