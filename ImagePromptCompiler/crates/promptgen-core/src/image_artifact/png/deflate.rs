use std::cell::Cell;

use super::PngError;

pub(super) fn inflate_zlib(bytes: &[u8], expected_limit: usize) -> Result<Vec<u8>, PngError> {
    if bytes.len() < 6 {
        return Err(PngError("truncated zlib stream".to_owned()));
    }
    let cmf = bytes[0];
    let flg = bytes[1];
    if cmf & 0x0f != 8 || cmf >> 4 > 7 {
        return Err(PngError(
            "unsupported zlib compression method/window".to_owned(),
        ));
    }
    if (u16::from(cmf) << 8 | u16::from(flg)) % 31 != 0 {
        return Err(PngError("invalid zlib header checksum".to_owned()));
    }
    if flg & 0x20 != 0 {
        return Err(PngError(
            "zlib preset dictionaries are unsupported".to_owned(),
        ));
    }
    let checksum_offset = bytes.len() - 4;
    let expected_adler =
        u32::from_be_bytes(bytes[checksum_offset..].try_into().expect("four bytes"));
    let mut reader = BitReader::new(&bytes[2..checksum_offset]);
    let mut output = Vec::with_capacity(expected_limit.min(1024 * 1024));
    loop {
        let final_block = reader.read_bits(1)? != 0;
        let block_type = reader.read_bits(2)?;
        match block_type {
            0 => decode_stored(&mut reader, &mut output, expected_limit)?,
            1 => {
                let (literal, distance) = fixed_tables()?;
                decode_compressed(
                    &mut reader,
                    &literal,
                    &distance,
                    &mut output,
                    expected_limit,
                )?;
            }
            2 => {
                let (literal, distance) = dynamic_tables(&mut reader)?;
                decode_compressed(
                    &mut reader,
                    &literal,
                    &distance,
                    &mut output,
                    expected_limit,
                )?;
            }
            _ => return Err(PngError("reserved DEFLATE block type".to_owned())),
        }
        if final_block {
            break;
        }
    }
    reader.align_byte();
    if reader.byte_position() != reader.len() {
        return Err(PngError("trailing bytes inside DEFLATE payload".to_owned()));
    }
    if adler32(&output) != expected_adler {
        return Err(PngError("Adler-32 mismatch".to_owned()));
    }
    Ok(output)
}

fn decode_stored(
    reader: &mut BitReader<'_>,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), PngError> {
    reader.align_byte();
    let len = reader.read_u16_le()?;
    let nlen = reader.read_u16_le()?;
    if len != !nlen {
        return Err(PngError(
            "stored DEFLATE block length complement mismatch".to_owned(),
        ));
    }
    let bytes = reader.read_aligned_bytes(len as usize)?;
    if output
        .len()
        .checked_add(bytes.len())
        .is_none_or(|value| value > limit)
    {
        return Err(PngError(
            "decoded DEFLATE output exceeds expected limit".to_owned(),
        ));
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn fixed_tables() -> Result<(Huffman, Huffman), PngError> {
    let mut literal_lengths = vec![0_u8; 288];
    literal_lengths[..144].fill(8);
    literal_lengths[144..256].fill(9);
    literal_lengths[256..280].fill(7);
    literal_lengths[280..].fill(8);
    let distance_lengths = vec![5_u8; 32];
    Ok((
        Huffman::new(&literal_lengths)?,
        Huffman::new(&distance_lengths)?,
    ))
}

fn dynamic_tables(reader: &mut BitReader<'_>) -> Result<(Huffman, Huffman), PngError> {
    let literal_count = reader.read_bits(5)? as usize + 257;
    let distance_count = reader.read_bits(5)? as usize + 1;
    let code_count = reader.read_bits(4)? as usize + 4;
    let order = [
        16_usize, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_lengths = vec![0_u8; 19];
    for index in 0..code_count {
        code_lengths[order[index]] = reader.read_bits(3)? as u8;
    }
    let code_table = Huffman::new(&code_lengths)?;
    let total = literal_count + distance_count;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        match code_table.decode(reader)? {
            0..=15 => lengths.push(code_table.last_symbol() as u8),
            16 => {
                let previous = *lengths
                    .last()
                    .ok_or_else(|| PngError("repeat code 16 has no previous length".to_owned()))?;
                let count = reader.read_bits(2)? as usize + 3;
                extend_lengths(&mut lengths, previous, count, total)?;
            }
            17 => {
                let count = reader.read_bits(3)? as usize + 3;
                extend_lengths(&mut lengths, 0, count, total)?;
            }
            18 => {
                let count = reader.read_bits(7)? as usize + 11;
                extend_lengths(&mut lengths, 0, count, total)?;
            }
            symbol => return Err(PngError(format!("invalid code-length symbol {symbol}"))),
        }
    }
    if lengths[256] == 0 {
        return Err(PngError(
            "dynamic literal table has no end-of-block symbol".to_owned(),
        ));
    }
    let literal = Huffman::new(&lengths[..literal_count])?;
    let distance = Huffman::new(&lengths[literal_count..])?;
    Ok((literal, distance))
}

fn extend_lengths(
    lengths: &mut Vec<u8>,
    value: u8,
    count: usize,
    total: usize,
) -> Result<(), PngError> {
    if lengths
        .len()
        .checked_add(count)
        .is_none_or(|value| value > total)
    {
        return Err(PngError("code-length repeat exceeds table size".to_owned()));
    }
    lengths.extend(std::iter::repeat_n(value, count));
    Ok(())
}

fn decode_compressed(
    reader: &mut BitReader<'_>,
    literal: &Huffman,
    distance: &Huffman,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), PngError> {
    const LENGTH_BASE: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DISTANCE_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    loop {
        let symbol = literal.decode(reader)?;
        match symbol {
            0..=255 => {
                if output.len() >= limit {
                    return Err(PngError(
                        "decoded DEFLATE output exceeds expected limit".to_owned(),
                    ));
                }
                output.push(symbol as u8);
            }
            256 => return Ok(()),
            257..=285 => {
                let index = symbol as usize - 257;
                let length = LENGTH_BASE[index] + reader.read_bits(LENGTH_EXTRA[index])? as usize;
                let distance_symbol = distance.decode(reader)? as usize;
                if distance_symbol >= DISTANCE_BASE.len() {
                    return Err(PngError("reserved DEFLATE distance symbol".to_owned()));
                }
                let back = DISTANCE_BASE[distance_symbol]
                    + reader.read_bits(DISTANCE_EXTRA[distance_symbol])? as usize;
                if back == 0 || back > output.len() {
                    return Err(PngError(
                        "DEFLATE distance points before output start".to_owned(),
                    ));
                }
                if output
                    .len()
                    .checked_add(length)
                    .is_none_or(|value| value > limit)
                {
                    return Err(PngError(
                        "decoded DEFLATE output exceeds expected limit".to_owned(),
                    ));
                }
                for _ in 0..length {
                    let byte = output[output.len() - back];
                    output.push(byte);
                }
            }
            _ => {
                return Err(PngError(
                    "reserved DEFLATE literal/length symbol".to_owned(),
                ));
            }
        }
    }
}

/// Canonical Huffman decoding table.
///
/// Codes of one bit length are consecutive integers, so a decoder never has to
/// search: `counts` says how many codes each length holds and `symbols` lists them
/// in canonical order, which turns "which symbol is this code" into one subtraction
/// and one index. The previous table scanned every code of the current length for
/// each decoded symbol — up to 288 comparisons per symbol on a path that decodes
/// millions of symbols for a single full-size PNG.
pub(super) struct Huffman {
    counts: Vec<u16>,
    symbols: Vec<u16>,
    max_len: u8,
    last: Cell<u16>,
}

impl Huffman {
    pub(super) fn new(lengths: &[u8]) -> Result<Self, PngError> {
        let max_len = lengths.iter().copied().max().unwrap_or(0);
        if max_len == 0 || max_len > 15 {
            return Err(PngError("empty or overlong Huffman table".to_owned()));
        }
        let mut counts = vec![0_u16; max_len as usize + 1];
        for &length in lengths {
            if length != 0 {
                counts[length as usize] += 1;
            }
        }
        let mut left = 1_i32;
        for &count in counts.iter().skip(1) {
            left = (left << 1) - i32::from(count);
            if left < 0 {
                return Err(PngError("oversubscribed Huffman table".to_owned()));
            }
        }
        // An incomplete table leaves valid-looking bit patterns undefined, so a corrupt
        // stream could decode to a symbol the encoder never wrote. DEFLATE permits
        // exactly one incomplete case — a single-symbol distance table — and nothing else.
        let assigned: u32 = counts.iter().skip(1).map(|count| u32::from(*count)).sum();
        if left > 0 && assigned != 1 {
            return Err(PngError("incomplete Huffman table".to_owned()));
        }
        // Offset of each length's first symbol inside the canonical symbol list.
        let mut offsets = vec![0_usize; max_len as usize + 2];
        for length in 1..=max_len as usize {
            offsets[length + 1] = offsets[length] + counts[length] as usize;
        }
        let mut symbols = vec![0_u16; offsets[max_len as usize + 1]];
        let mut cursor = offsets.clone();
        for (symbol, &length) in lengths.iter().enumerate() {
            if length == 0 {
                continue;
            }
            symbols[cursor[length as usize]] = symbol as u16;
            cursor[length as usize] += 1;
        }
        Ok(Self {
            counts,
            symbols,
            max_len,
            last: Cell::new(0),
        })
    }

    fn decode(&self, reader: &mut BitReader<'_>) -> Result<u16, PngError> {
        let mut code = 0_i32;
        let mut first = 0_i32;
        let mut index = 0_usize;
        for length in 1..=self.max_len as usize {
            code |= reader.read_bits(1)? as i32;
            let count = i32::from(self.counts[length]);
            if code - first < count {
                let symbol = self.symbols[index + (code - first) as usize];
                self.last.set(symbol);
                return Ok(symbol);
            }
            index += count as usize;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(PngError("invalid Huffman code".to_owned()))
    }

    fn last_symbol(&self) -> u16 {
        self.last.get()
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte: 0,
            bit: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, PngError> {
        if count > 16 {
            return Err(PngError("bit read exceeds 16 bits".to_owned()));
        }
        let mut value = 0_u32;
        for index in 0..count {
            if self.byte >= self.bytes.len() {
                return Err(PngError("truncated DEFLATE bitstream".to_owned()));
            }
            let bit = (self.bytes[self.byte] >> self.bit) & 1;
            value |= u32::from(bit) << index;
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.byte += 1;
            }
        }
        Ok(value)
    }

    fn align_byte(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.byte += 1;
        }
    }

    fn read_u16_le(&mut self) -> Result<u16, PngError> {
        let bytes = self.read_aligned_bytes(2)?;
        Ok(u16::from_le_bytes(bytes.try_into().expect("two bytes")))
    }

    fn read_aligned_bytes(&mut self, count: usize) -> Result<&'a [u8], PngError> {
        if self.bit != 0 {
            return Err(PngError("unaligned byte read".to_owned()));
        }
        let end = self
            .byte
            .checked_add(count)
            .ok_or_else(|| PngError("DEFLATE offset overflow".to_owned()))?;
        if end > self.bytes.len() {
            return Err(PngError("truncated DEFLATE byte sequence".to_owned()));
        }
        let bytes = &self.bytes[self.byte..end];
        self.byte = end;
        Ok(bytes)
    }

    const fn byte_position(&self) -> usize {
        self.byte
    }

    const fn len(&self) -> usize {
        self.bytes.len()
    }
}

pub(super) fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for chunk in bytes.chunks(5_552) {
        for byte in chunk {
            a += u32::from(*byte);
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}
