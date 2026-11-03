use super::ui_error;
use narrative_courier_contracts::ArtifactRef;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(super) struct DeliveryPreview {
    pub(super) mime: String,
    pub(super) path: String,
    pub(super) text: Option<String>,
    pub(super) data_url: Option<String>,
}

pub(super) fn from_verified_artifact(
    artifact: &ArtifactRef,
    bytes: Vec<u8>,
) -> Result<DeliveryPreview, String> {
    let mime = artifact
        .mime
        .clone()
        .ok_or_else(|| "delivery artifact has no MIME metadata for preview".to_owned())?;
    let text_mode = mime.starts_with("text/") || mime == "application/json";
    let (text, data_url) = if text_mode {
        (Some(String::from_utf8(bytes).map_err(ui_error)?), None)
    } else {
        (
            None,
            Some(format!("data:{};base64,{}", mime, base64_encode(&bytes))),
        )
    };
    Ok(DeliveryPreview {
        mime,
        path: artifact.path.clone(),
        text,
        data_url,
    })
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
