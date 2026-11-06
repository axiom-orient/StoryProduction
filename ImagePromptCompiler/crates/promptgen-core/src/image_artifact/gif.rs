//! Bounded GIF raster/timing validation. No provider or animation-loop authority.
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GifInfo {
    pub width: u32,
    pub height: u32,
    pub delays_ms: Vec<u32>,
}
#[derive(Debug)]
pub struct GifError(pub String);
impl fmt::Display for GifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for GifError {}
type Result<T> = std::result::Result<T, GifError>;
fn need(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(GifError(message.into()))
    }
}
struct Reader<'a> {
    raw: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or_else(|| GifError("GIF offset overflow".into()))?;
        let bytes = self
            .raw
            .get(self.offset..end)
            .ok_or_else(|| GifError("truncated GIF".into()))?;
        self.offset = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from(u16::from_le_bytes(
            self.take(2)?.try_into().unwrap(),
        )))
    }
    fn blocks(&mut self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            let n = usize::from(self.byte()?);
            if n == 0 {
                return Ok(out);
            }
            out.extend_from_slice(self.take(n)?);
        }
    }
}

pub fn inspect(raw: &[u8]) -> Result<GifInfo> {
    need(raw.len() <= 50 * 1024 * 1024, "GIF file budget exceeded")?;
    let mut r = Reader { raw, offset: 0 };
    need(
        matches!(r.take(6)?, b"GIF87a" | b"GIF89a"),
        "GIF signature required",
    )?;
    let width = r.word()?;
    let height = r.word()?;
    need(
        width > 0 && height > 0 && u64::from(width) * u64::from(height) <= 16_777_216,
        "GIF canvas budget",
    )?;
    let flags = r.byte()?;
    r.take(2)?;
    let global_colors = if flags & 0x80 != 0 {
        2_usize << (flags & 7)
    } else {
        0
    };
    r.take(global_colors * 3)?;
    let mut delays_ms = Vec::new();
    let mut delay = None;
    let mut transparent = None;
    loop {
        match r.byte()? {
            0x3b => {
                need(r.offset == raw.len(), "GIF trailing bytes")?;
                break;
            }
            0x21 => match r.byte()? {
                0xf9 => {
                    need(
                        delay.is_none() && r.byte()? == 4,
                        "invalid GIF control block",
                    )?;
                    let control = r.byte()?;
                    need((control >> 2) & 7 <= 3, "unsupported GIF disposal")?;
                    delay = Some(r.word()? * 10);
                    let index = r.byte()?;
                    transparent = (control & 1 != 0).then_some(index);
                    need(r.byte()? == 0, "unterminated GIF control block")?;
                }
                0xfe | 0xff => {
                    r.blocks()?;
                }
                _ => return Err(GifError("unsupported GIF extension".into())),
            },
            0x2c => {
                need(delays_ms.len() < 16, "GIF frame limit exceeded")?;
                let left = r.word()?;
                let top = r.word()?;
                let w = r.word()?;
                let h = r.word()?;
                need(
                    w > 0 && h > 0 && left + w <= width && top + h <= height,
                    "GIF frame outside canvas",
                )?;
                let flags = r.byte()?;
                let colors = if flags & 0x80 != 0 {
                    let colors = 2_usize << (flags & 7);
                    r.take(colors * 3)?;
                    colors
                } else {
                    global_colors
                };
                need(
                    colors > 0 && transparent.is_none_or(|n| usize::from(n) < colors),
                    "GIF palette invalid",
                )?;
                let minimum = r.byte()?;
                let data = r.blocks()?;
                decode_lzw(&data, minimum, (w * h) as usize, colors)?;
                let ms = delay
                    .take()
                    .ok_or_else(|| GifError("GIF frame delay required".into()))?;
                need(
                    (20..=2000).contains(&ms),
                    "GIF delay outside local 20..2000 ms profile",
                )?;
                delays_ms.push(ms);
                transparent = None;
                need(
                    u64::from(width) * u64::from(height) * delays_ms.len() as u64 <= 16_777_216,
                    "GIF total pixel budget",
                )?;
            }
            _ => return Err(GifError("invalid GIF block".into())),
        }
    }
    need(
        delays_ms.len() >= 2 && delay.is_none(),
        "animated GIF requires at least two complete frames",
    )?;
    Ok(GifInfo {
        width,
        height,
        delays_ms,
    })
}

fn decode_lzw(data: &[u8], minimum: u8, expected: usize, colors: usize) -> Result<()> {
    need((2..=8).contains(&minimum), "invalid GIF LZW minimum")?;
    let clear = 1_usize << minimum;
    let end = clear + 1;
    let base = || {
        (0..clear)
            .map(|n| vec![n as u8])
            .chain([Vec::new(), Vec::new()])
            .collect::<Vec<_>>()
    };
    let mut table = base();
    let mut bits = usize::from(minimum) + 1;
    let mut offset = 0;
    let mut previous: Option<Vec<u8>> = None;
    let mut count = 0_usize;
    let mut saw_clear = false;
    loop {
        need(offset + bits <= data.len() * 8, "truncated GIF LZW stream")?;
        let mut code = 0;
        for bit in 0..bits {
            code |= usize::from((data[(offset + bit) / 8] >> ((offset + bit) % 8)) & 1) << bit;
        }
        offset += bits;
        if code == clear {
            table = base();
            bits = usize::from(minimum) + 1;
            previous = None;
            saw_clear = true;
            continue;
        }
        need(saw_clear, "GIF LZW missing clear code")?;
        if code == end {
            return need(count == expected, "GIF decoded pixel count mismatch");
        }
        let entry = if let Some(value) = table.get(code) {
            value.clone()
        } else if code == table.len() {
            let mut value = previous
                .clone()
                .ok_or_else(|| GifError("invalid GIF first LZW code".into()))?;
            value.push(value[0]);
            value
        } else {
            return Err(GifError("invalid GIF LZW code".into()));
        };
        need(
            !entry.is_empty() && entry.iter().all(|n| usize::from(*n) < colors),
            "GIF palette index invalid",
        )?;
        count = count
            .checked_add(entry.len())
            .ok_or_else(|| GifError("GIF pixel overflow".into()))?;
        need(count <= expected, "GIF decompression exceeds frame")?;
        if let Some(mut old) = previous.take().filter(|_| table.len() < 4096) {
            old.push(entry[0]);
            table.push(old);
            if table.len() == 1 << bits && bits < 12 {
                bits += 1;
            }
        }
        previous = Some(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_multicolor_dictionary_growth_fixture() {
        // ffmpeg lavfi testsrc=32x32:rate=5:duration=1.8; no story/provider pixels.
        let info = inspect(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/codec-test-nineframes.gif"
        )))
        .unwrap();
        assert_eq!((info.width, info.height), (32, 32));
        assert_eq!(info.delays_ms, vec![200; 9]);
    }
    fn fixture() -> Vec<u8> {
        // Two generated codec-test frames. This is not image-provider evidence.
        let mut raw = b"GIF89a\x01\0\x01\0\x80\0\0\0\0\0\xff\xff\xff".to_vec();
        for pixel in [0x44, 0x4c] {
            raw.extend_from_slice(b"\x21\xf9\x04\0\x14\0\0\0\x2c\0\0\0\0\x01\0\x01\0\0\x02\x02");
            raw.extend_from_slice(&[pixel, 0x01, 0]);
        }
        raw.push(0x3b);
        raw
    }
    #[test]
    fn decodes_real_lzw_and_checks_timing_truncation_and_palette() {
        let raw = fixture();
        assert_eq!(
            inspect(&raw).unwrap(),
            GifInfo {
                width: 1,
                height: 1,
                delays_ms: vec![200, 200]
            }
        );
        for end in 0..raw.len() {
            assert!(inspect(&raw[..end]).is_err(), "truncation at {end}");
        }
        let mut corrupt = raw.clone();
        corrupt[39] = 0xff;
        assert!(inspect(&corrupt).is_err());
        assert!(inspect(b"\x89PNG\r\n\x1a\n").is_err());
        let mut still = raw[..42].to_vec();
        still.push(0x3b);
        assert!(inspect(&still).is_err());
    }
}
