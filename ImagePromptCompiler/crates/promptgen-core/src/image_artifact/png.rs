mod deflate;

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use self::deflate::inflate_zlib;
#[cfg(test)]
use self::deflate::{Huffman, adler32};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MAX_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_DECODED_BYTES: usize = 8_294_400 * 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PngInfo {
    pub width: u32,
    pub height: u32,
    pub decoded_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PngError(pub String);

impl fmt::Display for PngError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PngError {}

pub fn validate_file(path: &Path, expected: (u32, u32)) -> Result<PngInfo, PngError> {
    validate_file_with_expected(path, Some(expected))
}

pub fn inspect_file(path: &Path) -> Result<PngInfo, PngError> {
    validate_file_with_expected(path, None)
}

pub fn inspect_bytes(bytes: &[u8]) -> Result<PngInfo, PngError> {
    validate_bytes_with_expected(bytes, None)
}

fn validate_file_with_expected(
    path: &Path,
    expected: Option<(u32, u32)>,
) -> Result<PngInfo, PngError> {
    let mut file = File::open(path).map_err(|error| PngError(format!("open failed: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| PngError(format!("metadata failed: {error}")))?;
    if metadata.len() > MAX_FILE_BYTES as u64 {
        return Err(PngError(format!(
            "file exceeds limit: {} > {MAX_FILE_BYTES}",
            metadata.len()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| PngError(format!("read failed: {error}")))?;
    validate_bytes_with_expected(&bytes, expected)
}

pub fn validate_bytes(bytes: &[u8], expected: (u32, u32)) -> Result<PngInfo, PngError> {
    validate_bytes_with_expected(bytes, Some(expected))
}

fn validate_bytes_with_expected(
    bytes: &[u8],
    expected: Option<(u32, u32)>,
) -> Result<PngInfo, PngError> {
    if bytes.len() < PNG_SIGNATURE.len() || &bytes[..8] != PNG_SIGNATURE {
        return Err(PngError("invalid PNG signature".to_owned()));
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(PngError("PNG exceeds file-size limit".to_owned()));
    }

    let mut offset = 8_usize;
    let mut ihdr: Option<(u32, u32, u8, u8, u8)> = None;
    let mut idat = Vec::new();
    let mut seen_idat = false;
    let mut idat_ended = false;
    let mut seen_iend = false;
    let mut seen_plte = false;
    let mut chunk_index = 0_usize;

    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err(PngError(format!("truncated chunk header at byte {offset}")));
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"));
        let length =
            usize::try_from(length).map_err(|_| PngError("chunk length overflow".to_owned()))?;
        let chunk_type: [u8; 4] = bytes[offset + 4..offset + 8]
            .try_into()
            .expect("four bytes");
        let data_start = offset + 8;
        let data_end = data_start
            .checked_add(length)
            .ok_or_else(|| PngError("chunk length overflow".to_owned()))?;
        let crc_end = data_end
            .checked_add(4)
            .ok_or_else(|| PngError("chunk CRC offset overflow".to_owned()))?;
        if crc_end > bytes.len() {
            return Err(PngError(format!(
                "truncated {} chunk",
                chunk_name(chunk_type)
            )));
        }
        let expected_crc =
            u32::from_be_bytes(bytes[data_end..crc_end].try_into().expect("four bytes"));
        let actual_crc = crc32(&bytes[offset + 4..data_end]);
        if actual_crc != expected_crc {
            return Err(PngError(format!(
                "CRC mismatch in {} chunk: expected {expected_crc:08x}, actual {actual_crc:08x}",
                chunk_name(chunk_type)
            )));
        }
        let data = &bytes[data_start..data_end];

        match &chunk_type {
            b"IHDR" => {
                if chunk_index != 0 || ihdr.is_some() || length != 13 {
                    return Err(PngError(
                        "IHDR must be the first and unique 13-byte chunk".to_owned(),
                    ));
                }
                let width = u32::from_be_bytes(data[0..4].try_into().expect("four bytes"));
                let height = u32::from_be_bytes(data[4..8].try_into().expect("four bytes"));
                let bit_depth = data[8];
                let color_type = data[9];
                let compression = data[10];
                let filter = data[11];
                let interlace = data[12];
                if width == 0 || height == 0 {
                    return Err(PngError("PNG dimensions must be positive".to_owned()));
                }
                if let Some(expected) = expected
                    && (width, height) != expected
                {
                    return Err(PngError(format!(
                        "dimension mismatch: expected {}x{}, actual {width}x{height}",
                        expected.0, expected.1
                    )));
                }
                validate_color_mode(bit_depth, color_type)?;
                if compression != 0 || filter != 0 {
                    return Err(PngError(
                        "unsupported PNG compression or filter method".to_owned(),
                    ));
                }
                if interlace != 0 {
                    return Err(PngError(
                        "interlaced PNG is rejected because the adapter requires complete non-interlaced scanline verification"
                            .to_owned(),
                    ));
                }
                ihdr = Some((width, height, bit_depth, color_type, interlace));
            }
            b"PLTE" => {
                if ihdr.is_none()
                    || seen_idat
                    || seen_plte
                    || data.is_empty()
                    || !data.len().is_multiple_of(3)
                {
                    return Err(PngError("invalid PLTE placement or length".to_owned()));
                }
                if data.len() > 768 {
                    return Err(PngError("PLTE has more than 256 entries".to_owned()));
                }
                seen_plte = true;
            }
            b"IDAT" => {
                if ihdr.is_none() || idat_ended {
                    return Err(PngError(
                        "IDAT chunks must be consecutive and follow IHDR".to_owned(),
                    ));
                }
                seen_idat = true;
                idat.extend_from_slice(data);
                if idat.len() > MAX_FILE_BYTES {
                    return Err(PngError("combined IDAT data exceeds limit".to_owned()));
                }
            }
            b"IEND" => {
                if !seen_idat || seen_iend || length != 0 {
                    return Err(PngError(
                        "IEND must be unique, empty, and follow IDAT".to_owned(),
                    ));
                }
                seen_iend = true;
                if crc_end != bytes.len() {
                    return Err(PngError("trailing bytes after IEND".to_owned()));
                }
            }
            _ => {
                if seen_idat {
                    idat_ended = true;
                }
                if chunk_type[0].is_ascii_uppercase() {
                    return Err(PngError(format!(
                        "unknown critical chunk {}",
                        chunk_name(chunk_type)
                    )));
                }
            }
        }
        offset = crc_end;
        chunk_index += 1;
        if seen_iend {
            break;
        }
    }

    let Some((width, height, bit_depth, color_type, _)) = ihdr else {
        return Err(PngError("missing IHDR".to_owned()));
    };
    if color_type == 3 && !seen_plte {
        return Err(PngError("indexed-color PNG is missing PLTE".to_owned()));
    }
    if !seen_iend {
        return Err(PngError("missing IEND".to_owned()));
    }
    let row_bytes = row_bytes(width, bit_depth, color_type)?;
    let expected_decoded = (row_bytes + 1)
        .checked_mul(height as usize)
        .ok_or_else(|| PngError("decoded-size overflow".to_owned()))?;
    if expected_decoded > MAX_DECODED_BYTES {
        return Err(PngError(format!(
            "decoded scanlines exceed limit: {expected_decoded} > {MAX_DECODED_BYTES}"
        )));
    }
    let decoded = inflate_zlib(&idat, expected_decoded)?;
    if decoded.len() != expected_decoded {
        return Err(PngError(format!(
            "decoded scanline length mismatch: expected {expected_decoded}, actual {}",
            decoded.len()
        )));
    }
    for row in 0..height as usize {
        let filter = decoded[row * (row_bytes + 1)];
        if filter > 4 {
            return Err(PngError(format!(
                "invalid filter byte {filter} in row {row}"
            )));
        }
    }
    Ok(PngInfo {
        width,
        height,
        decoded_bytes: decoded.len(),
    })
}

fn validate_color_mode(bit_depth: u8, color_type: u8) -> Result<(), PngError> {
    let valid = match color_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        2 => matches!(bit_depth, 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        4 | 6 => matches!(bit_depth, 8 | 16),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(PngError(format!(
            "invalid bit-depth/color-type combination: bit_depth={bit_depth}, color_type={color_type}"
        )))
    }
}

fn row_bytes(width: u32, bit_depth: u8, color_type: u8) -> Result<usize, PngError> {
    let channels = match color_type {
        0 | 3 => 1_u64,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return Err(PngError("unsupported color type".to_owned())),
    };
    let bits = u64::from(width)
        .checked_mul(channels)
        .and_then(|value| value.checked_mul(u64::from(bit_depth)))
        .ok_or_else(|| PngError("row-size overflow".to_owned()))?;
    usize::try_from(bits.div_ceil(8)).map_err(|_| PngError("row-size overflow".to_owned()))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn chunk_name(chunk_type: [u8; 4]) -> String {
    String::from_utf8_lossy(&chunk_type).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_complete_stored_block_png() {
        let png = make_png(2, 2);
        let info = validate_bytes(&png, (2, 2)).expect("valid PNG");
        assert_eq!(info.decoded_bytes, 18);
    }

    #[test]
    fn rejects_crc_and_dimension_mismatch() {
        let mut png = make_png(2, 2);
        png[20] ^= 1;
        assert!(validate_bytes(&png, (2, 2)).unwrap_err().0.contains("CRC"));
        let png = make_png(2, 2);
        assert!(
            validate_bytes(&png, (1, 1))
                .unwrap_err()
                .0
                .contains("dimension")
        );
    }

    /// The two fixtures cover one fixed and one dynamic block. The six shipped
    /// representative cards add production-encoder coverage.
    #[test]
    fn every_catalog_png_decodes_and_matches_its_declared_dimensions() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/assets");
        let mut checked = 0usize;
        let mut stack = vec![root];
        while let Some(directory) = stack.pop() {
            for entry in std::fs::read_dir(&directory).expect("catalog assets directory") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("png") {
                    continue;
                }
                let bytes = std::fs::read(&path).expect("read png");
                let declared = (
                    u32::from_be_bytes(bytes[16..20].try_into().expect("width")),
                    u32::from_be_bytes(bytes[20..24].try_into().expect("height")),
                );
                let info = validate_bytes(&bytes, declared)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                assert_eq!((info.width, info.height), declared, "{}", path.display());
                checked += 1;
            }
        }
        assert_eq!(checked, 6, "every published catalog card must be decoded");
    }

    #[test]
    fn incomplete_huffman_tables_are_rejected() {
        // Two two-bit codes leave half the code space undefined; only a single-symbol
        // table may be incomplete.
        assert!(Huffman::new(&[2, 2]).is_err());
        assert!(Huffman::new(&[1, 1]).is_ok());
        assert!(Huffman::new(&[1]).is_ok());
        assert!(Huffman::new(&[1, 1, 1]).is_err());
    }

    #[test]
    fn validates_fixed_and_dynamic_huffman_pngs() {
        let fixed = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../promptgen-codex/tests/fixtures/fixed-32x32.png"
        ));
        let dynamic = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../promptgen-codex/tests/fixtures/dynamic-32x32.png"
        ));
        assert_eq!(
            validate_bytes(fixed, (32, 32)).unwrap().decoded_bytes,
            4_128
        );
        assert_eq!(
            validate_bytes(dynamic, (32, 32)).unwrap().decoded_bytes,
            4_128
        );
    }

    #[test]
    fn rejects_corrupt_deflate_even_with_recomputed_chunk_crc() {
        let mut png = make_png(2, 2);
        let idat = locate_chunk(&png, b"IDAT");
        png[idat.0 + 7] ^= 0x40;
        let crc = crc32(&png[idat.0 - 4..idat.1]);
        png[idat.1..idat.1 + 4].copy_from_slice(&crc.to_be_bytes());
        assert!(validate_bytes(&png, (2, 2)).is_err());
    }

    fn make_png(width: u32, height: u32) -> Vec<u8> {
        let row_bytes = width as usize * 4;
        let mut raw = Vec::new();
        for row in 0..height as usize {
            raw.push(0);
            for column in 0..row_bytes {
                raw.push((row * row_bytes + column) as u8);
            }
        }
        let mut zlib = vec![0x78, 0x01, 0x01];
        let len = raw.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(&raw);
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        let mut png = PNG_SIGNATURE.to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        append_chunk(&mut png, b"IHDR", &ihdr);
        append_chunk(&mut png, b"IDAT", &zlib);
        append_chunk(&mut png, b"IEND", &[]);
        png
    }

    fn append_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = png.len();
        png.extend_from_slice(kind);
        png.extend_from_slice(data);
        let crc = crc32(&png[start..]);
        png.extend_from_slice(&crc.to_be_bytes());
    }

    fn locate_chunk(bytes: &[u8], kind: &[u8; 4]) -> (usize, usize) {
        let mut offset = 8;
        loop {
            let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            if &bytes[offset + 4..offset + 8] == kind {
                return (offset + 8, offset + 8 + length);
            }
            offset += 12 + length;
        }
    }
}
