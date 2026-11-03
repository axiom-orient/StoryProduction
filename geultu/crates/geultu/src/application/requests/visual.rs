use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::application) struct VisualPlanRequest {
    pub(in crate::application) project_id: String,
    pub(in crate::application) scene_id: String,
    pub(in crate::application) plan_id: Option<String>,
    pub(in crate::application) command_id: Option<String>,
}

pub(in crate::application) struct VisualAdmitPackage {
    pub(in crate::application) input: crate::visual_run::AdmitPackageInput,
    pub(in crate::application) command_id: Option<String>,
}

impl<'de> Deserialize<'de> for VisualAdmitPackage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let serde_json::Value::Object(mut fields) = serde_json::Value::deserialize(deserializer)?
        else {
            return Err(D::Error::custom(
                "visual admission arguments must be an object",
            ));
        };
        let command_id = match fields.remove("command_id") {
            Some(value) => serde_json::from_value(value).map_err(D::Error::custom)?,
            None => None,
        };
        let input =
            serde_json::from_value(serde_json::Value::Object(fields)).map_err(D::Error::custom)?;
        Ok(Self { input, command_id })
    }
}

/// visual.admit_package response: commit fields plus preflight/run proof.
#[derive(Serialize)]
pub(in crate::application) struct VisualAdmitPackageResponse {
    pub(in crate::application) project_id: String,
    pub(in crate::application) run_id: String,
    pub(in crate::application) preflight: Vec<crate::visual_run::PreflightCheck>,
    pub(in crate::application) run: crate::contracts::VisualRunState,
    pub(in crate::application) receipt: crate::contracts::ValidationReceipt,
    pub(in crate::application) revision: u64,
    pub(in crate::application) event_count: usize,
    pub(in crate::application) idempotent: bool,
}
