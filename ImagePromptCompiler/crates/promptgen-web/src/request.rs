use promptgen_core::InterviewRequest;
use promptgen_core::json::{
    DecodeError, JsonValue, into_object, parse_with_limits, reject_unknown, take_optional,
    take_required, take_string,
};

use super::WebError;

pub(super) fn decode_interview(
    body: &[u8],
    max_body_bytes: usize,
) -> Result<InterviewRequest, WebError> {
    let text = std::str::from_utf8(body)
        .map_err(|_| WebError::new("JSON_ENCODING", "JSON body must be UTF-8"))?;
    let value = parse_with_limits(text, max_body_bytes, 96)
        .map_err(|error| WebError::new("JSON_PARSE", error.to_string()))?;
    InterviewRequest::from_json(value)
        .map_err(|error| WebError::new("INTERVIEW_SCHEMA", error.to_string()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GenerationMode {
    PromptOnly,
    CodexImagegen,
}

impl GenerationMode {
    pub(super) fn parse(value: &str) -> Result<Self, DecodeError> {
        match value {
            "prompt-only" => Ok(Self::PromptOnly),
            "codex-imagegen" => Ok(Self::CodexImagegen),
            _ => Err(DecodeError::new(
                "$.mode",
                "expected prompt-only or codex-imagegen",
            )),
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::PromptOnly => "prompt-only",
            Self::CodexImagegen => "codex-imagegen",
        }
    }
}

pub(super) struct GenerateRequest {
    pub(super) interview: InterviewRequest,
    pub(super) mode: GenerationMode,
    pub(super) output_name: Option<String>,
}

impl GenerateRequest {
    pub(super) fn decode(body: &[u8], max_body_bytes: usize) -> Result<Self, WebError> {
        let text = std::str::from_utf8(body)
            .map_err(|_| WebError::new("JSON_ENCODING", "JSON body must be UTF-8"))?;
        let value = parse_with_limits(text, max_body_bytes, 96)
            .map_err(|error| WebError::new("JSON_PARSE", error.to_string()))?;
        let mut fields = into_object(value, "$generate")
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let interview_value = take_required(&mut fields, "interview", "$generate")
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let interview = InterviewRequest::from_json(interview_value)
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let mode = take_string(&mut fields, "mode", "$generate")
            .and_then(|value| GenerationMode::parse(&value))
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let output_name = match take_optional(&mut fields, "output_name") {
            None | Some(JsonValue::Null) => None,
            Some(JsonValue::String(value)) => Some(value),
            Some(_) => {
                return Err(WebError::new(
                    "GENERATE_SCHEMA",
                    "$.output_name must be a string or null",
                ));
            }
        };
        reject_unknown(fields, "$generate")
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        Ok(Self {
            interview,
            mode,
            output_name,
        })
    }
}
