//! Closed MCP input DTOs and schemas; durable request constraints come from their producer.

mod schema;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};

fn nonempty_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("text must not be empty"));
    }
    Ok(value)
}
fn optional_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    nonempty_text(deserializer).map(Some)
}
fn positive_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = u32::deserialize(deserializer)?;
    if value == 0 {
        return Err(serde::de::Error::custom("target_duration must be positive"));
    }
    Ok(value)
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectParams {
    /// Project name exactly as returned by list_projects.
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunDeliveryParams {
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub project: String,
    /// Producer-owned DeliveryRequest. Its request id is immutable.
    #[schemars(schema_with = "schema::delivery_request")]
    pub request: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScreenplayFormat {
    Fountain,
    Fdx,
    Docx,
}

impl ScreenplayFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Fountain => "fountain",
            Self::Fdx => "fdx",
            Self::Docx => "docx",
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportScreenplayParams {
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub project: String,
    pub format: ScreenplayFormat,
    /// Optional bare filename ending in the selected format extension inside the project's output directory. Omit for the default filename.
    #[serde(default, deserialize_with = "optional_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub output_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportScreenplayParams {
    /// Omit to derive the project name from the title.
    #[serde(default, deserialize_with = "optional_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub project: Option<String>,
    /// Human-approved synopsis; the screenplay is reference material only.
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub synopsis: String,
    /// Local path to a Fountain or FDX file readable by this server.
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub script_path: String,
    /// Omit to use the script file stem.
    #[serde(default, deserialize_with = "optional_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub title: Option<String>,
    #[serde(deserialize_with = "positive_duration")]
    #[schemars(schema_with = "schema::positive_duration")]
    pub target_duration: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSynopsisParams {
    /// Omit to derive the project name from the reviewed handoff title.
    #[serde(default, deserialize_with = "optional_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub project: Option<String>,
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub handoff_root: String,
    #[serde(deserialize_with = "nonempty_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub handoff_path: String,
    /// If supplied, must equal the reviewed handoff title.
    #[serde(default, deserialize_with = "optional_text")]
    #[schemars(schema_with = "schema::nonempty_text")]
    pub title: Option<String>,
    #[serde(deserialize_with = "positive_duration")]
    #[schemars(schema_with = "schema::positive_duration")]
    pub target_duration: u32,
}
