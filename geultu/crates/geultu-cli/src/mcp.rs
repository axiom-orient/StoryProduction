use geultu::{Application, Error};
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

// MCP carries JSON-lines, so this transport bound must cover six-byte JSON
// escaping of the 20 MiB voice corpus plus a bounded request envelope. The
// domain still rejects corpora above its 20 MiB product limit.
const MAX_MESSAGE_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const SUPPORTED_PROTOCOLS: &[&str] = &["2025-11-25", "2025-06-18", "2024-11-05"];

pub fn run(app: Application) -> Result<(), String> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut session = SessionState::New;
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .by_ref()
            .take((MAX_MESSAGE_BYTES as u64).saturating_add(1))
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            break;
        }
        if bytes > MAX_MESSAGE_BYTES {
            return Err(format!("MCP message exceeds {MAX_MESSAGE_BYTES} bytes"));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(error) => {
                write_response(
                    &mut writer,
                    &rpc_error(
                        Value::Null,
                        -32700,
                        "Parse error",
                        Some(json!({"detail": error.to_string()})),
                    ),
                )?;
                continue;
            }
        };
        let outcome = handle_message(&app, &mut session, value);
        match outcome {
            Outcome::Response(response) => write_response(&mut writer, &response)?,
            Outcome::Notification => {}
        }
    }
    Ok(())
}

fn write_response(writer: &mut impl Write, response: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, response).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

enum Outcome {
    Response(Value),
    Notification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    New,
    AwaitingInitialized,
    Ready,
}

fn handle_message(app: &Application, session: &mut SessionState, request: Value) -> Outcome {
    let Some(object) = request.as_object() else {
        return Outcome::Response(rpc_error(Value::Null, -32600, "Invalid Request", None));
    };
    let id = object.get("id");
    let valid_id = |value: &Value| value.is_string() || value.is_i64() || value.is_u64();
    let response_id = id
        .filter(|value| valid_id(value))
        .cloned()
        .unwrap_or(Value::Null);
    if id.is_some_and(|value| !valid_id(value)) {
        return Outcome::Response(rpc_error(
            Value::Null,
            -32600,
            "Invalid Request",
            Some(json!({"detail":"id must be a string or integer"})),
        ));
    }
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Outcome::Response(rpc_error(
            response_id,
            -32600,
            "Invalid Request",
            Some(json!({"detail":"jsonrpc must be 2.0"})),
        ));
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return Outcome::Response(rpc_error(
            response_id,
            -32600,
            "Invalid Request",
            Some(json!({"detail":"method is required"})),
        ));
    };
    let empty_params = json!({});
    let params = object.get("params").unwrap_or(&empty_params);
    if !params.is_object() {
        return if id.is_none() {
            Outcome::Notification
        } else {
            Outcome::Response(rpc_error(
                response_id,
                -32602,
                "Invalid params",
                Some(json!({"detail":"params must be an object"})),
            ))
        };
    }
    if id.is_none() {
        if method == "notifications/initialized" && *session == SessionState::AwaitingInitialized {
            *session = SessionState::Ready;
        }
        // Unknown/progress/cancel notifications have no response or provider-cancellation guarantee.
        return Outcome::Notification;
    }
    let id = response_id;
    match method {
        "initialize" => {
            if *session != SessionState::New {
                return Outcome::Response(rpc_error(id, -32600, "Already initialized", None));
            }
            let Some(requested) = params.get("protocolVersion").and_then(Value::as_str) else {
                return Outcome::Response(rpc_error(
                    id,
                    -32602,
                    "Invalid params",
                    Some(json!({"detail":"protocolVersion is required"})),
                ));
            };
            let client = params.get("clientInfo");
            let nonempty = |field: &str| {
                client
                    .and_then(|value| value.get(field))
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
            };
            if requested.is_empty()
                || !params.get("capabilities").is_some_and(Value::is_object)
                || !client.is_some_and(Value::is_object)
                || !nonempty("name")
                || !nonempty("version")
            {
                return Outcome::Response(rpc_error(
                    id,
                    -32602,
                    "Invalid params",
                    Some(
                        json!({"detail":"initialize requires protocolVersion, capabilities and clientInfo name/version"}),
                    ),
                ));
            }
            let selected = if SUPPORTED_PROTOCOLS.contains(&requested) {
                requested
            } else {
                SUPPORTED_PROTOCOLS[0]
            };
            *session = SessionState::AwaitingInitialized;
            Outcome::Response(rpc_result(
                id,
                json!({
                    "protocolVersion": selected,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "geultu", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": "Geultu owns canonical project state. AI output stays a non-canonical candidate until a human reviews and applies it through Studio or the trusted local CLI. candidate.subject is a read-only exact subject lookup; MCP cannot execute human-only edits, rejection, approval or handoff. Use command_id only when listed in the tool schema. Progress/cancel notifications do not cancel a synchronous provider call."
                }),
            ))
        }
        "ping" => Outcome::Response(rpc_result(id, json!({}))),
        "tools/list" => {
            if *session != SessionState::Ready {
                return Outcome::Response(not_initialized(id));
            }
            let tools: Vec<_> = app
                .tool_definitions()
                .into_iter()
                .filter(|tool| !geultu::application::is_human_only_tool(tool.name))
                .collect();
            Outcome::Response(rpc_result(id, json!({"tools": tools})))
        }
        "tools/call" => {
            if *session != SessionState::Ready {
                return Outcome::Response(not_initialized(id));
            }
            let name = match params.get("name").and_then(Value::as_str) {
                Some(name) => name,
                None => {
                    return Outcome::Response(rpc_error(
                        id,
                        -32602,
                        "Invalid params",
                        Some(json!({"detail":"name is required"})),
                    ))
                }
            };
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if geultu::application::is_human_only_tool(name) {
                return Outcome::Response(rpc_result(id, tool_error(Error::InvalidInput(
                    "human-only authoring edits, approvals, rejections and handoff must use Studio or the trusted local CLI".into(),
                ))));
            }
            match app.call(name, arguments) {
                Ok(result) => {
                    let text = match serde_json::to_string_pretty(&result) {
                        Ok(text) => text,
                        Err(error) => {
                            return Outcome::Response(rpc_error(
                                id,
                                -32603,
                                "Internal error",
                                Some(json!({"detail":error.to_string()})),
                            ))
                        }
                    };
                    Outcome::Response(rpc_result(
                        id,
                        json!({
                            "content": [{"type":"text", "text": text}],
                            "structuredContent": result,
                            "isError": false
                        }),
                    ))
                }
                Err(error) => Outcome::Response(rpc_result(id, tool_error(error))),
            }
        }
        _ => Outcome::Response(rpc_error(
            id,
            -32601,
            "Method not found",
            Some(json!({"method": method})),
        )),
    }
}

fn tool_error(error: Error) -> Value {
    let body = error.body();
    let text = error.to_string();
    json!({
        "content": [{"type":"text", "text": text}],
        "structuredContent": {"error": body},
        "isError": true
    })
}

fn not_initialized(id: Value) -> Value {
    rpc_error(id, -32002, "Server not initialized", None)
}
fn rpc_result(id: Value, result: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "result":result})
}
fn rpc_error(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut error = json!({"code":code, "message":message});
    if let Some(data) = data {
        error["data"] = data;
    }
    json!({"jsonrpc":"2.0", "id":id, "error":error})
}

#[cfg(test)]
mod tests {
    use super::*;
    use geultu::Workspace;
    use std::fs;

    const JSON_ESCAPE_EXPANSION: usize = 6;
    const VOICE_REQUEST_ENVELOPE_BYTES: usize = 7 * 1024 * 1024;
    const VOICE_REQUEST_LINE_END_BYTES: usize = 1;

    #[test]
    fn initialize_then_list_tools() {
        let root = std::env::temp_dir().join(format!("geultu-mcp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let app = Application::from_workspace(Workspace::init(&root).unwrap());
        let mut session = SessionState::New;
        let init = handle_message(
            &app,
            &mut session,
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}}),
        );
        match init {
            Outcome::Response(value) => assert_eq!(value["result"]["serverInfo"]["name"], "geultu"),
            _ => panic!("expected response"),
        }
        let _ = handle_message(
            &app,
            &mut session,
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        );
        let list = handle_message(
            &app,
            &mut session,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        );
        match list {
            Outcome::Response(value) => {
                let tools = value["result"]["tools"].as_array().unwrap();
                assert!(tools.len() >= 20);
                assert!(tools.iter().any(|tool| tool["name"] == "candidate.subject"));
                for human_only in [
                    "authoring.review",
                    "candidate.reject",
                    "changeset.approve",
                    "changeset.reject",
                    "synopsis.export_reviewed_handoff",
                ] {
                    assert!(!tools.iter().any(|tool| tool["name"] == human_only));
                }
            }
            _ => panic!("expected response"),
        }
        let blocked_decision = handle_message(
            &app,
            &mut session,
            json!({
                "jsonrpc":"2.0",
                "id":3,
                "method":"tools/call",
                "params":{"name":"changeset.approve","arguments":{}}
            }),
        );
        match blocked_decision {
            Outcome::Response(value) => assert_eq!(value["result"]["isError"], true),
            _ => panic!("expected a human-only decision refusal"),
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn transport_message_covers_worst_case_voice_json_envelope() {
        let worst_case = geultu::voice::MAX_VOICE_CORPUS_BYTES * JSON_ESCAPE_EXPANSION
            + VOICE_REQUEST_ENVELOPE_BYTES
            + VOICE_REQUEST_LINE_END_BYTES;
        assert!(MAX_MESSAGE_BYTES >= worst_case);
    }
}
