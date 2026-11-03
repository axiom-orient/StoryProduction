use std::collections::BTreeMap;

use crate::diagnostic::PromptKind;

mod contracts;
mod image;

pub use contracts::{
    InterviewOption, InterviewOutcome, InterviewQuestion, InterviewReady, InterviewRequest,
    InterviewRequestValue, InterviewStatus, interview_schema,
};

pub fn run_interview(request: &InterviewRequest) -> InterviewOutcome {
    match request.kind {
        PromptKind::Image => image::run(request),
    }
}

pub(crate) fn normalized_answers(request: &InterviewRequest) -> BTreeMap<String, String> {
    request
        .answers
        .iter()
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

pub(crate) fn answer<'a>(answers: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    answers
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{JsonValue, parse};

    #[test]
    fn request_rejects_empty_brief_and_noncanonical_answer_keys() {
        for source in [
            r#"{"kind":"image","brief":"   ","answers":{}}"#,
            r#"{"kind":"image","brief":"x","answers":{" image.category ":"C1"}}"#,
            r#"{"kind":"image","brief":"x","answers":{"":"C1"}}"#,
        ] {
            let value = parse(source).expect("JSON");
            assert!(InterviewRequest::from_json(value).is_err(), "{source}");
        }
    }

    #[test]
    fn strict_request_rejects_non_string_answer() {
        let value = parse(r#"{"kind":"image","brief":"x","answers":{"x":1}}"#).unwrap();
        assert!(InterviewRequest::from_json(value).is_err());
    }

    #[test]
    fn schema_is_valid_json() {
        let rendered = interview_schema().to_pretty_string();
        assert!(crate::json::parse(&rendered).is_ok());
    }

    #[test]
    fn schema_rejects_noncanonical_answer_keys_like_the_runtime_decoder() {
        let schema = interview_schema();
        let properties = schema
            .as_object()
            .and_then(|root| root.get("properties"))
            .and_then(JsonValue::as_object)
            .expect("properties");
        let pattern = properties
            .get("answers")
            .and_then(JsonValue::as_object)
            .and_then(|answers| answers.get("propertyNames"))
            .and_then(JsonValue::as_object)
            .and_then(|property_names| property_names.get("pattern"))
            .and_then(JsonValue::as_str);
        assert_eq!(pattern, Some("^\\S(?:[\\s\\S]*\\S)?$"));
    }

    #[test]
    fn schema_requires_string_answer_values_like_the_runtime_decoder() {
        let schema = interview_schema();
        let properties = schema
            .as_object()
            .and_then(|root| root.get("properties"))
            .and_then(JsonValue::as_object)
            .expect("properties");
        let answer_properties = properties
            .get("answers")
            .and_then(JsonValue::as_object)
            .and_then(|answers| answers.get("additionalProperties"))
            .and_then(JsonValue::as_object)
            .expect("answer value schema");
        assert_eq!(
            answer_properties.get("type").and_then(JsonValue::as_str),
            Some("string")
        );
    }
}
