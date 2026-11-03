//! Routes public tool names to the existing workspace and domain owners.

pub(super) mod generation;
pub(super) mod narrative;
pub(super) mod refinement;
pub(super) mod review;
pub(super) mod visual;
pub(super) mod voice;
pub(super) mod workspace;

use super::Application;
use crate::error::{Error, Result};
use serde_json::Value;

fn unknown_tool(name: &str) -> Error {
    Error::NotFound(format!(
        "unknown tool '{name}'; run `geultu tools` or MCP tools/list to see registered tools"
    ))
}

impl Application {
    pub fn call(&self, name: &str, arguments: Value) -> Result<Value> {
        let tool = super::tools::lookup(name).ok_or_else(|| unknown_tool(name))?;
        let object = arguments.as_object().ok_or_else(|| {
            Error::InvalidInput(format!("tool '{name}' arguments must be an object"))
        })?;
        let properties = tool.definition.input_schema["properties"]
            .as_object()
            .ok_or_else(|| {
                Error::Corrupt(format!("tool '{name}' has no object input properties"))
            })?;
        for key in object.keys() {
            if !properties.contains_key(key) {
                return Err(Error::InvalidInput(format!(
                    "tool '{name}' received unknown field `{key}`"
                )));
            }
        }
        if let Some(required) = tool.definition.input_schema["required"].as_array() {
            for field in required {
                let key = field.as_str().ok_or_else(|| {
                    Error::Corrupt(format!("tool '{name}' has an invalid required field"))
                })?;
                if !object.contains_key(key) {
                    return Err(Error::InvalidInput(format!(
                        "tool '{name}' requires field '{key}'"
                    )));
                }
            }
        }
        (tool.handler)(self, name, arguments)
    }
}
