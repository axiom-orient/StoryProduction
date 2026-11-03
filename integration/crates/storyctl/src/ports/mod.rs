//! Narrow public-owner ports; no domain store access is available here.
use crate::error::Result;
use serde_json::Value;
use std::path::Path;
pub trait WritingOwner {
    fn initialize(&self) -> Result<()>;
    fn call(&self, operation: &str, input: &Value) -> Result<Value>;
}
pub trait NarrativeOwner {
    fn text(&self, command: &str, project: &str, arguments: &[String]) -> Result<String>;
    fn tool(&self, lane: &str, arguments: &[String], project_root: &Path) -> Result<Value>;
}
