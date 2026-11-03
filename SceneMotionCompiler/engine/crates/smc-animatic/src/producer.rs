//! Consumer hashing follows each producer's immutable profile.
//! SAC's Python-v1 compact bytes preserve integral floats; NVC and SMC use JCS.
use crate::io::{self, Result};
use serde_json::Value;

pub fn validate_hash(name: &str, value: &Value) -> Result<()> {
    let expected = if matches!(name, "animatic-assets-v1" | "animatic-assets-review-v1") {
        let mut body = value.clone();
        body.as_object_mut()
            .ok_or("OBJECT_REQUIRED")?
            .remove("contentHash");
        // The pinned SAC schemas have closed ASCII keys and only bounded integer
        // dimensions as numeric fields. Keep their legacy float spelling intact.
        io::sha(&serde_json::to_vec(&body).map_err(|e| e.to_string())?)
    } else {
        io::seal(value.clone())?["contentHash"]
            .as_str()
            .ok_or("CONTENT_HASH_REQUIRED")?
            .to_owned()
    };
    io::check(value["contentHash"] == expected, "CONTENT_HASH_CHANGED")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn sac_integral_float_profile_is_not_resealed_as_jcs() {
        let mut value = json!({"width":1.0,"height":1.0});
        value["contentHash"] = io::sha(br#"{"height":1.0,"width":1.0}"#).into();
        validate_hash("animatic-assets-v1", &value).unwrap();
        assert!(validate_hash("animatic-plan-v1", &value).is_err());
        value["width"] = json!(2.0);
        assert!(validate_hash("animatic-assets-v1", &value).is_err());
    }
}
