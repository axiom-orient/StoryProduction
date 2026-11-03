//! Delivery-owned validation and import of external image assets.

use super::super::{verify_released_package, DeliveryEngine, ImportedDeliveryAsset};
use crate::encoding::base64_encode;
use image::{ImageFormat, ImageReader};
use narrative_courier_contracts::{AssetSource, NarrativePackage};
use narrative_courier_runtime::store::{lock_project, sha256_bytes};
use narrative_courier_runtime::{Error, Result};
use std::io::Cursor;
use std::path::Path;

use super::{publish_blob, read_source_bytes, MAX_IMAGE_BYTES};
const MAX_IMPORTED_IMAGE_PIXELS: u64 = 40_000_000;

pub(in crate::engine) fn validate_import_dimensions(width: u32, height: u32) -> Result<()> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| {
            Error::Contract("INVALID_IMAGE: pixel count overflows validation domain".into())
        })?;
    if width == 0 || height == 0 {
        return Err(Error::Contract(
            "INVALID_IMAGE: dimensions must be positive".into(),
        ));
    }
    if pixels > MAX_IMPORTED_IMAGE_PIXELS {
        return Err(Error::Contract(format!(
            "INVALID_IMAGE: decoded pixel count {pixels} exceeds {MAX_IMPORTED_IMAGE_PIXELS}"
        )));
    }
    Ok(())
}

pub(in crate::engine) fn inspect_imported_image(bytes: &[u8]) -> Result<(ImageFormat, u32, u32)> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| Error::Contract(format!("INVALID_IMAGE: cannot identify image bytes: {e}")))?;
    let format = reader
        .format()
        .ok_or_else(|| Error::Contract("INVALID_IMAGE: image format is unknown".into()))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(Error::Contract(format!(
            "UNSUPPORTED_CAPABILITY: imported image format {format:?} is not PNG, JPEG, or WEBP"
        )));
    }
    match format {
        ImageFormat::Png => {
            let decoder = image::codecs::png::PngDecoder::new(Cursor::new(bytes))
                .map_err(|e| Error::Contract(format!("INVALID_IMAGE: PNG decoder failed: {e}")))?;
            if decoder.is_apng().map_err(|e| {
                Error::Contract(format!("INVALID_IMAGE: PNG animation check failed: {e}"))
            })? {
                return Err(Error::Contract(
                    "UNSUPPORTED_CAPABILITY: animated PNG assets are not supported".into(),
                ));
            }
        }
        ImageFormat::WebP => {
            let decoder = image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))
                .map_err(|e| Error::Contract(format!("INVALID_IMAGE: WEBP decoder failed: {e}")))?;
            if decoder.has_animation() {
                return Err(Error::Contract(
                    "UNSUPPORTED_CAPABILITY: animated WEBP assets are not supported".into(),
                ));
            }
        }
        ImageFormat::Jpeg => {}
        _ => unreachable!("format filtered above"),
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| Error::Contract(format!("INVALID_IMAGE: cannot read dimensions: {e}")))?;
    validate_import_dimensions(width, height)?;
    image::load_from_memory_with_format(bytes, format)
        .map_err(|e| Error::Contract(format!("INVALID_IMAGE: decode failed: {e}")))?;
    Ok((format, width, height))
}

pub(in crate::engine) fn imported_format_details(
    format: ImageFormat,
) -> (&'static str, &'static str) {
    match format {
        ImageFormat::Png => ("png", "image/png"),
        ImageFormat::Jpeg => ("jpg", "image/jpeg"),
        ImageFormat::WebP => ("webp", "image/webp"),
        _ => unreachable!("format filtered by inspect_imported_image"),
    }
}

fn import_image(
    project: &Path,
    package: &NarrativePackage,
    source: &Path,
) -> Result<ImportedDeliveryAsset> {
    let _lock = lock_project(project)?;
    verify_released_package(project, package)?;

    let source = source
        .canonicalize()
        .map_err(|e| Error::Contract(format!("INVALID_IMAGE: cannot canonicalize source: {e}")))?;
    let bytes = read_source_bytes(&source, Some(MAX_IMAGE_BYTES))?;
    let name = source
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error::Contract("imported image filename must be UTF-8".into()))?
        .to_string();
    let (format, width, height) = inspect_imported_image(&bytes)?;
    let sha256 = sha256_bytes(&bytes);
    let (extension, mime) = imported_format_details(format);
    let relative = format!("visual/assets/imports/{sha256}.{extension}");
    publish_blob(project, &relative, &bytes, &sha256)?;
    Ok(ImportedDeliveryAsset {
        asset_id: format!("IMG-{sha256}"),
        name,
        source: AssetSource {
            path: relative,
            sha256: sha256.clone(),
            mime: Some(mime.into()),
        },
        preview_data_url: format!("data:{mime};base64,{}", base64_encode(&bytes)),
        width,
        height,
        size_bytes: bytes.len() as u64,
    })
}

impl DeliveryEngine {
    pub fn import_asset(
        project: &Path,
        package: &NarrativePackage,
        source: &Path,
    ) -> Result<ImportedDeliveryAsset> {
        import_image(project, package, source)
    }
}

#[cfg(test)]
mod tests {
    use super::{inspect_imported_image, validate_import_dimensions};
    use image::ImageFormat;
    use std::io::Cursor;

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::new_rgb8(1, 1);
        let mut bytes = Cursor::new(Vec::new());
        image
            .write_to(&mut bytes, ImageFormat::Png)
            .expect("encode test PNG");
        bytes.into_inner()
    }

    #[test]
    fn decoder_accepts_real_png_bytes_without_considering_extension() {
        let bytes = png_bytes();
        let (format, width, height) = inspect_imported_image(&bytes).expect("valid PNG");
        assert_eq!(format, ImageFormat::Png);
        assert_eq!((width, height), (1, 1));
    }

    #[test]
    fn decoder_rejects_spoofed_or_invalid_bytes() {
        let err = inspect_imported_image(b"not an image").expect_err("invalid bytes must fail");
        assert!(err.to_string().contains("INVALID_IMAGE"));
    }

    #[test]
    fn pixel_cap_rejects_large_dimensions_with_checked_arithmetic() {
        let err = validate_import_dimensions(6_400, 6_251).expect_err("40 MP cap must fail");
        assert!(err.to_string().contains("decoded pixel count"));
        assert!(validate_import_dimensions(6_400, 6_250).is_ok());
    }
}
