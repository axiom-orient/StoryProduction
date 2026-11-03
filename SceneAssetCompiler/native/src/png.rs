//! Bounded PNG inspection for SAC review. Decodes existing bytes; never creates pixels.
use crate::{check, Error, Result};
use flate2::{Decompress, FlushDecompress, Status};

const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;
pub struct Observation {
    pub width: u32,
    pub height: u32,
    pub has_transparent_pixels: bool,
    pub alpha_max: Option<u32>,
}

fn u32be(raw: &[u8]) -> u32 {
    u32::from_be_bytes(raw.try_into().unwrap())
}
fn u16be(raw: &[u8]) -> u32 {
    u16::from_be_bytes(raw.try_into().unwrap()) as u32
}

pub fn inspect(data: &[u8]) -> Result<Observation> {
    check(
        data.len() >= 33 && &data[..8] == b"\x89PNG\r\n\x1a\n",
        "PNG_BAD_SIGNATURE",
    )?;
    let mut offset = 8usize;
    let mut header = None;
    let mut idat = Vec::new();
    let mut transparency = None;
    let mut palette = None;
    let mut iend = false;
    while offset < data.len() {
        check(data.len() - offset >= 12, "PNG_TRUNCATED_CHUNK")?;
        let length = u32be(&data[offset..offset + 4]) as usize;
        let kind = &data[offset + 4..offset + 8];
        let end = offset
            .checked_add(length)
            .and_then(|v| v.checked_add(12))
            .ok_or_else(|| Error::refused("PNG_TRUNCATED_CHUNK"))?;
        check(end <= data.len(), "PNG_TRUNCATED_CHUNK")?;
        let payload = &data[offset + 8..end - 4];
        check(
            crc32fast::hash(&data[offset + 4..end - 4]) == u32be(&data[end - 4..end]),
            "PNG_BAD_CRC",
        )?;
        check(offset != 8 || kind == b"IHDR", "PNG_IHDR_NOT_FIRST")?;
        match kind {
            b"IHDR" => {
                check(header.is_none() && length == 13, "PNG_BAD_IHDR")?;
                header = Some((
                    u32be(&payload[..4]),
                    u32be(&payload[4..8]),
                    payload[8],
                    payload[9],
                    payload[10],
                    payload[11],
                    payload[12],
                ));
            }
            b"IDAT" => idat.extend_from_slice(payload),
            b"tRNS" => transparency = Some(payload),
            b"PLTE" => palette = Some(payload),
            b"IEND" => {
                check(length == 0, "PNG_BAD_IEND")?;
                iend = true;
                offset = end;
                break;
            }
            _ => {}
        }
        offset = end;
    }
    check(iend && offset == data.len(), "PNG_BAD_TERMINATION")?;
    let (width, height, depth, color, compression, filter, interlace) =
        header.ok_or_else(|| Error::refused("PNG_REQUIRED_CHUNK_MISSING"))?;
    check(!idat.is_empty(), "PNG_REQUIRED_CHUNK_MISSING")?;
    check(width > 0 && height > 0, "PNG_EMPTY_CANVAS")?;
    check(
        match color {
            0 => [1, 2, 4, 8, 16].contains(&depth),
            2 | 4 | 6 => [8, 16].contains(&depth),
            3 => [1, 2, 4, 8].contains(&depth),
            _ => false,
        },
        "PNG_UNSUPPORTED_COLOR_TYPE",
    )?;
    if color == 3 {
        let p = palette.ok_or_else(|| Error::refused("PNG_BAD_PALETTE"))?;
        check(
            !p.is_empty() && p.len() % 3 == 0 && p.len() <= 768 && p.len() / 3 <= 1usize << depth,
            "PNG_BAD_PALETTE",
        )?;
        check(
            transparency.is_none_or(|t| t.len() <= p.len() / 3),
            "PNG_BAD_TRNS",
        )?;
    } else if let Some(t) = transparency {
        check(
            match color {
                0 => t.len() == 2,
                2 => t.len() == 6,
                _ => false,
            },
            "PNG_BAD_TRNS",
        )?;
    }
    check(compression == 0 && filter == 0, "PNG_UNSUPPORTED_METHOD")?;
    check(interlace == 0, "PNG_INTERLACE_UNSUPPORTED")?;
    let channels: usize = match color {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!(),
    };
    let bits = (width as usize)
        .checked_mul(channels)
        .and_then(|v| v.checked_mul(depth as usize))
        .ok_or_else(|| Error::refused("PNG_DECODE_LIMIT"))?;
    let row_bytes = bits
        .checked_add(7)
        .ok_or_else(|| Error::refused("PNG_DECODE_LIMIT"))?
        / 8;
    let bytes_per_pixel = (channels * depth as usize).div_ceil(8).max(1);
    let expected = (height as usize)
        .checked_mul(row_bytes + 1)
        .ok_or_else(|| Error::refused("PNG_DECODE_LIMIT"))?;
    check(expected <= MAX_DECODED_BYTES, "PNG_DECODE_LIMIT")?;
    let mut decoder = Decompress::new(true);
    let mut decoded = vec![0u8; expected + 1];
    let status = decoder
        .decompress(&idat, &mut decoded, FlushDecompress::Finish)
        .map_err(|e| Error::refused(format!("PNG_DEFLATE:{e}")))?;
    check(decoder.total_out() <= expected as u64, "PNG_DECODE_LIMIT")?;
    check(
        status == Status::StreamEnd
            && decoder.total_out() == expected as u64
            && decoder.total_in() == idat.len() as u64,
        "PNG_SCANLINE_SIZE",
    )?;
    let mut prior = vec![0u8; row_bytes];
    let mut has_transparent_pixels = false;
    let mut alpha_max = None;
    let opaque = (1u32 << depth) - 1;
    let mut observe = |value: u32, fully_opaque: u32| {
        has_transparent_pixels |= value < fully_opaque;
        alpha_max = Some(alpha_max.map_or(value, |max: u32| max.max(value)));
    };
    for scanline in decoded[..expected].chunks_exact(row_bytes + 1) {
        let method = scanline[0];
        check(method <= 4, "PNG_FILTER")?;
        let mut row = scanline[1..].to_vec();
        for index in 0..row_bytes {
            let left = if index >= bytes_per_pixel {
                row[index - bytes_per_pixel]
            } else {
                0
            };
            let up = prior[index];
            let upper_left = if index >= bytes_per_pixel {
                prior[index - bytes_per_pixel]
            } else {
                0
            };
            let value = match method {
                1 => left,
                2 => up,
                3 => ((left as u16 + up as u16) / 2) as u8,
                4 => paeth(left, up, upper_left),
                _ => 0,
            };
            row[index] = row[index].wrapping_add(value);
        }
        match color {
            4 | 6 => {
                let sample_bytes = depth as usize / 8;
                for pixel in row.chunks_exact(channels * sample_bytes) {
                    let sample = &pixel[(channels - 1) * sample_bytes..];
                    observe(
                        if depth == 16 {
                            u16be(sample)
                        } else {
                            sample[0] as u32
                        },
                        opaque,
                    );
                }
            }
            3 => {
                let entries = palette.unwrap().len() / 3;
                let mask = ((1u16 << depth) - 1) as u8;
                for x in 0..width as usize {
                    let bit = x * depth as usize;
                    let index = ((row[bit / 8] >> (8 - depth as usize - bit % 8)) & mask) as usize;
                    check(index < entries, "PNG_PALETTE_INDEX_OUT_OF_RANGE")?;
                    observe(
                        transparency
                            .and_then(|t| t.get(index))
                            .copied()
                            .unwrap_or(255) as u32,
                        255,
                    );
                }
            }
            0 if transparency.is_some() => {
                let transparent = u16be(transparency.unwrap());
                check(transparent <= opaque, "PNG_BAD_TRNS_SAMPLE")?;
                for x in 0..width as usize {
                    let sample = if depth == 16 {
                        u16be(&row[x * 2..x * 2 + 2])
                    } else if depth == 8 {
                        row[x] as u32
                    } else {
                        let bit = x * depth as usize;
                        ((row[bit / 8] >> (8 - depth as usize - bit % 8)) as u32) & opaque
                    };
                    observe(if sample == transparent { 0 } else { opaque }, opaque);
                }
            }
            2 if transparency.is_some() => {
                let t = transparency.unwrap();
                let transparent = [u16be(&t[..2]), u16be(&t[2..4]), u16be(&t[4..6])];
                check(
                    transparent.iter().all(|v| *v <= opaque),
                    "PNG_BAD_TRNS_SAMPLE",
                )?;
                let sample_bytes = depth as usize / 8;
                for pixel in row.chunks_exact(3 * sample_bytes) {
                    let sample = if depth == 16 {
                        [u16be(&pixel[..2]), u16be(&pixel[2..4]), u16be(&pixel[4..6])]
                    } else {
                        [pixel[0] as u32, pixel[1] as u32, pixel[2] as u32]
                    };
                    observe(if sample == transparent { 0 } else { opaque }, opaque);
                }
            }
            _ => {}
        }
        prior = row;
    }
    Ok(Observation {
        width,
        height,
        has_transparent_pixels,
        alpha_max,
    })
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let (pa, pb, pc) = (
        (p - a as i32).abs(),
        (p - b as i32).abs(),
        (p - c as i32).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    fn chunk(output: &mut Vec<u8>, kind: &[u8], payload: &[u8]) {
        output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        let mut raw = kind.to_vec();
        raw.extend_from_slice(payload);
        output.extend_from_slice(&crc32fast::hash(&raw).to_be_bytes());
    }
    pub(crate) fn png(alpha: u8) -> Vec<u8> {
        let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(&mut output, b"IHDR", &ihdr);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&[0, 0, 0, 0, alpha]).unwrap();
        chunk(&mut output, b"IDAT", &encoder.finish().unwrap());
        chunk(&mut output, b"IEND", &[]);
        output
    }
    #[test]
    fn opacity_comes_from_decoded_pixels() {
        assert!(!inspect(&png(255)).unwrap().has_transparent_pixels);
        let transparent = inspect(&png(0)).unwrap();
        assert!(transparent.has_transparent_pixels);
        assert_eq!(transparent.alpha_max, Some(0));
        assert!(inspect(&png(128)).unwrap().has_transparent_pixels);
    }
    #[test]
    fn crc_and_trailing_bytes_are_rejected() {
        let mut raw = png(255);
        raw[45] ^= 1;
        assert!(inspect(&raw).is_err());
        let mut raw = png(255);
        raw.push(0);
        assert!(inspect(&raw).is_err());
    }
}
