mod decode;
mod parser;
mod value;

pub use self::decode::{
    DecodeError, expect_bool, expect_f64, expect_string, expect_string_array, expect_u64,
    into_array, into_object, reject_unknown, take_bool, take_f64, take_optional,
    take_optional_bool, take_optional_f64, take_optional_string, take_optional_string_array,
    take_optional_u64, take_required, take_string, take_string_array, take_u64,
};
pub use self::parser::{JsonError, parse, parse_with_limits};
pub use self::value::JsonValue;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_round_trips_unicode_and_numbers() {
        let source = r#"{"한글":"문자열","array":[null,true,-12.5e2,"\uD83D\uDE00"]}"#;
        let value = parse(source).expect("valid JSON");
        let reparsed = parse(&value.to_compact_string()).expect("serialized JSON");
        assert_eq!(value, reparsed);
    }

    #[test]
    fn rejects_duplicate_keys() {
        let error = parse(r#"{"a":1,"a":2}"#).expect_err("duplicate key must fail");
        assert!(error.message.contains("duplicate"));
    }

    #[test]
    fn rejects_trailing_commas_and_leading_zero() {
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\":1,}").is_err());
        assert!(parse("01").is_err());
    }

    #[test]
    fn enforces_depth_limit() {
        let error = parse_with_limits("[[[0]]]", 100, 1).expect_err("depth must fail");
        assert!(error.message.contains("depth"));
    }

    #[test]
    fn pretty_output_is_deterministic_and_sorted() {
        let value = JsonValue::object([("z", JsonValue::from(1_u64)), ("a", JsonValue::from("x"))]);
        assert_eq!(
            value.to_pretty_string(),
            "{\n  \"a\": \"x\",\n  \"z\": 1\n}\n"
        );
    }
}
