//! Data decodation. This comes after error correction and visual detection.
//!
//! It performs the inverse of the `encodation` module.
use super::encodation::{ascii, edifact, EncodationType, UNLATCH};
use alloc::{string::String, vec::Vec};

#[cfg(test)]
use alloc::vec;

#[cfg(test)]
mod tests;

mod eci;

#[derive(Debug, PartialEq)]
pub enum DataDecodingError {
    UnexpectedCharacter(&'static str, u8),
    NotImplemented(&'static str),
    UnexpectedEnd,
    CharsetError,
    /// An ECI code is not supported in raw data decoding
    ECICode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Reader<'a>(&'a [u8], usize);

impl<'a> Reader<'a> {
    fn pos(&self) -> usize {
        self.1 + 1
    }

    fn eat(&mut self) -> Result<u8, DataDecodingError> {
        if let Some((ch, rest)) = self.0.split_first() {
            self.1 += 1;
            self.0 = rest;
            Ok(*ch)
        } else {
            Err(DataDecodingError::UnexpectedEnd)
        }
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn peek(&self, i: usize) -> Option<u8> {
        self.0.get(i).cloned()
    }
}

/// Decode the data codewords of a Data Matrix.
pub fn decode_data(data: &[u8]) -> Result<Vec<u8>, DataDecodingError> {
    let (out, ecis) = decode_parts(data)?;
    if !ecis.is_empty() {
        Err(DataDecodingError::ECICode)
    } else {
        Ok(out)
    }
}

fn decode_parts(data: &[u8]) -> Result<(Vec<u8>, Vec<(usize, u32)>), DataDecodingError> {
    let mut data = Reader(data, 0);
    let mut mode = EncodationType::Ascii;
    let mut out = Vec::with_capacity(data.len());
    let mut ecis = Vec::new();

    while !data.is_empty() {
        let (rest, new_mode) = match mode {
            EncodationType::Ascii => decode_ascii(data, &mut out, &mut ecis)?,
            EncodationType::Base256 => decode_base256(data, &mut out)?,
            EncodationType::X12 => decode_x12(data, &mut out)?,
            EncodationType::Edifact => decode_edifact(data, &mut out)?,
            EncodationType::C40 => decode_c40_like(data, &mut out, BASE_C40, SHIFT3_C40)?,
            EncodationType::Text => decode_c40_like(data, &mut out, BASE_TEXT, SHIFT3_TEXT)?,
        };
        data = rest;
        mode = new_mode;
    }
    Ok((out, ecis))
}

/// Decode the data codewords of a Data Matrix as a string.
///
/// This recognizes has some ECI support. Be aware that
/// latin1 encoding is assumed if no ECI is there.
pub fn decode_str(data: &[u8]) -> Result<String, DataDecodingError> {
    let (out, ecis) = decode_parts(data)?;
    eci::convert(&out, &ecis)
}

fn derandomize_253_state(ch: u8, pos: usize) -> u8 {
    let pseudo_random = ((149 * pos) % 253) + 1;
    let tmp = ch as i16 - pseudo_random as i16;
    if tmp >= 1 {
        tmp as u8
    } else {
        (tmp + 254) as u8
    }
}

fn read_eci(mut data: Reader) -> Result<(Reader, u32), DataDecodingError> {
    let mut ch1 = data.eat()?;
    let eci = match ch1 {
        1..=127 => ch1 as u32 - 1,
        128..=191 => {
            let mut ch2 = data.eat()?;
            if !matches!(ch2, 1..=254) {
                return Err(DataDecodingError::UnexpectedCharacter("2nd after ECI", ch2));
            }
            ch2 -= 1;
            ch1 -= 128;
            (ch1 as u32) * 254 + ch2 as u32 + 127
        }
        192..=207 => {
            let mut ch2 = data.eat()?;
            if !matches!(ch2, 1..=254) {
                return Err(DataDecodingError::UnexpectedCharacter("2nd after ECI", ch2));
            }
            let mut ch3 = data.eat()?;
            if !matches!(ch2, 1..=254) {
                return Err(DataDecodingError::UnexpectedCharacter("3rd after ECI", ch3));
            }
            ch1 -= 192;
            ch2 -= 1;
            ch3 -= 1;
            (ch1 as u32) * 64516 + (ch2 as u32) * 254 + ch3 as u32 + 16383
        }
        _ => return Err(DataDecodingError::UnexpectedCharacter("1st after ECI", ch1)),
    };
    Ok((data, eci))
}

fn decode_ascii<'a>(
    mut data: Reader<'a>,
    out: &mut Vec<u8>,
    ecis: &mut Vec<(usize, u32)>,
) -> Result<(Reader<'a>, EncodationType), DataDecodingError> {
    let mut upper_shift = false;
    while let Ok(ch) = data.eat() {
        match ch {
            ch @ 1..=128 => {
                if upper_shift {
                    out.push(ch + 127);
                    upper_shift = false;
                } else {
                    out.push(ch - 1);
                }
            }
            ascii::PAD => {
                // eat rest, check padding format
                while let Ok(ch) = data.eat() {
                    let ch = derandomize_253_state(ch, data.pos() - 1);
                    if ch != ascii::PAD {
                        return Err(DataDecodingError::UnexpectedCharacter(
                            "non-padding char in padding area",
                            ch,
                        ));
                    }
                }
                return Ok((data, EncodationType::Ascii));
            }
            ch @ 130..=229 => {
                let digit = ch - 130;
                out.push(b'0' + (digit / 10));
                out.push(b'0' + (digit % 10));
            }
            ascii::LATCH_C40 => return Ok((data, EncodationType::C40)),
            ascii::LATCH_BASE256 => return Ok((data, EncodationType::Base256)),
            232 => return Err(DataDecodingError::NotImplemented("FNC1")),
            233 => return Err(DataDecodingError::NotImplemented("Structured Append")),
            234 => return Err(DataDecodingError::NotImplemented("Reader Programming")),
            ascii::UPPER_SHIFT => {
                upper_shift = true;
            }
            236 => return Err(DataDecodingError::NotImplemented("05 Macro")),
            237 => return Err(DataDecodingError::NotImplemented("06 Macro")),
            ascii::LATCH_X12 => return Ok((data, EncodationType::X12)),
            ascii::LATCH_TEXT => return Ok((data, EncodationType::Text)),
            ascii::LATCH_EDIFACT => return Ok((data, EncodationType::Edifact)),
            ascii::ECI => {
                let (rest, eci) = read_eci(data)?;
                data = rest;
                ecis.push((out.len(), eci));
            }
            ch => {
                return Err(DataDecodingError::UnexpectedCharacter(
                    "illegal in ascii",
                    ch,
                ))
            }
        }
    }
    Ok((data, EncodationType::Ascii))
}

fn derandomize_255_state(ch: u8, pos: usize) -> u8 {
    let pseudo_random = ((149 * pos) % 255) + 1;
    let tmp = ch as i16 - pseudo_random as i16;
    if tmp >= 0 {
        tmp as u8
    } else {
        (tmp + 256) as u8
    }
}

fn decode_base256<'a>(
    mut data: Reader<'a>,
    out: &mut Vec<u8>,
) -> Result<(Reader<'a>, EncodationType), DataDecodingError> {
    let length = if let Ok(ch1) = data.eat() {
        let ch1 = derandomize_255_state(ch1, data.pos() - 1) as usize;
        if ch1 == 0 {
            data.len()
        } else if ch1 < 250 {
            ch1
        } else {
            let ch2 = data.eat()?;
            let ch2 = derandomize_255_state(ch2, data.pos() - 1) as usize;
            250 * (ch1 - 249) + ch2
        }
    } else {
        return Err(DataDecodingError::UnexpectedEnd);
    };
    for _ in 0..length {
        if let Ok(ch) = data.eat() {
            out.push(derandomize_255_state(ch, data.pos() - 1));
        } else {
            return Err(DataDecodingError::UnexpectedEnd);
        }
    }
    Ok((data, EncodationType::Ascii))
}

fn dec_edifcat_char(ch: u8) -> u8 {
    if (ch & 0b10_0000) != 0 {
        ch
    } else {
        ch | 0b0100_0000
    }
}

fn decode_edifact<'a>(
    mut data: Reader<'a>,
    out: &mut Vec<u8>,
) -> Result<(Reader<'a>, EncodationType), DataDecodingError> {
    while !data.is_empty() {
        if data.len() <= 2 {
            // rest is encoded as ASCII
            break;
        }
        if data.peek(0).unwrap() >> 2 == edifact::UNLATCH {
            data.eat().unwrap();
            break;
        }
        let mut chunk: u32 = (data.eat().unwrap() as u32) << 16;
        let val = (chunk >> 18) as u8;
        if val == edifact::UNLATCH {
            break;
        }
        out.push(dec_edifcat_char(val));

        if let Ok(ch) = data.eat() {
            chunk |= (ch as u32) << 8;
            let val = ((chunk >> 12) & 0b11_1111) as u8;
            if val == edifact::UNLATCH {
                break;
            }
            out.push(dec_edifcat_char(val));

            if let Ok(ch) = data.eat() {
                chunk |= ch as u32;
                let val = ((chunk >> 6) & 0b11_1111) as u8;
                if val == edifact::UNLATCH {
                    break;
                }
                out.push(dec_edifcat_char(val));

                let val = (chunk & 0b11_1111) as u8;
                if val == edifact::UNLATCH {
                    break;
                }
                out.push(dec_edifcat_char(val));
            }
        }
    }
    Ok((data, EncodationType::Ascii))
}

fn decode_c40_tuple(a: u8, b: u8) -> (u8, u8, u8) {
    let mut full = ((a as u16) << 8) + b as u16 - 1;
    let tmp = full / 1600;
    let c1 = tmp as u8;
    full -= tmp * 1600;
    let tmp = full / 40;
    (c1, tmp as u8, (full - tmp * 40) as u8)
}

fn dec_x12_val(ch: u8) -> Result<u8, DataDecodingError> {
    match ch {
        0 => Ok(13),
        1 => Ok(42),
        2 => Ok(62),
        3 => Ok(b' '),
        ch @ 4..=13 => Ok(b'0' + (ch - 4)),
        ch @ 14..=39 => Ok(b'A' + (ch - 14)),
        ch => Err(DataDecodingError::UnexpectedCharacter("not x12", ch)),
    }
}

fn decode_x12<'a>(
    mut data: Reader<'a>,
    out: &mut Vec<u8>,
) -> Result<(Reader<'a>, EncodationType), DataDecodingError> {
    while data.len() > 1 {
        let first = data.eat().unwrap();
        if first == UNLATCH {
            break;
        }
        let second = data.eat().unwrap();
        let (c1, c2, c3) = decode_c40_tuple(first, second);

        out.push(dec_x12_val(c1)?);
        out.push(dec_x12_val(c2)?);
        out.push(dec_x12_val(c3)?);
    }
    if data.peek(0) == Some(UNLATCH) {
        // single UNLATCH at end of data
        let _ = data.eat().unwrap();
    }
    Ok((data, EncodationType::Ascii))
}

const BASE_C40: &[u8; 37] = b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const SHIFT3_C40: &[u8; 32] = b"`abcdefghijklmnopqrstuvwxyz{|}~\x7f";

const BASE_TEXT: &[u8; 37] = b" 0123456789abcdefghijklmnopqrstuvwxyz";
const SHIFT3_TEXT: &[u8; 32] = b"`ABCDEFGHIJKLMNOPQRSTUVWXYZ{|}~\x7f";

const SHIFT2: &[u8] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_";

fn decode_c40_like<'a>(
    mut data: Reader<'a>,
    out: &mut Vec<u8>,
    map_base: &[u8; 37],
    map_shift3: &[u8; 32],
) -> Result<(Reader<'a>, EncodationType), DataDecodingError> {
    let mut shift = 0;
    let mut upper_shift = false;
    while data.len() > 1 {
        let first = data.eat().unwrap();
        if first == UNLATCH {
            break;
        }
        let (c1, c2, c3) = decode_c40_tuple(first, data.eat().unwrap());
        for ch in [c1, c2, c3].iter().cloned() {
            if shift == 0 {
                match ch {
                    ch @ 0..=2 => shift = ch + 1,
                    ch @ 3..=39 => {
                        let text = map_base[ch as usize - 3];
                        if upper_shift {
                            out.push(text + 128);
                            upper_shift = false;
                        } else {
                            out.push(text);
                        }
                    }
                    ch => {
                        return Err(DataDecodingError::UnexpectedCharacter(
                            "not in base c40/text",
                            ch,
                        ))
                    }
                }
            } else if shift == 1 {
                match ch {
                    ch @ 0..=31 => {
                        if upper_shift {
                            out.push(ch + 128);
                            upper_shift = false;
                        } else {
                            out.push(ch);
                        }
                    }
                    ch => {
                        return Err(DataDecodingError::UnexpectedCharacter(
                            "not in shift1 c40/text",
                            ch,
                        ))
                    }
                }
                shift = 0;
            } else if shift == 2 {
                match ch {
                    ch @ 0..=26 => {
                        let text = SHIFT2[ch as usize];
                        if upper_shift {
                            out.push(text + 128);
                            upper_shift = false;
                        } else {
                            out.push(text);
                        }
                    }
                    27 => return Err(DataDecodingError::NotImplemented("FNC1 in C40/Text")),
                    30 => upper_shift = true,
                    _ => {
                        return Err(DataDecodingError::UnexpectedCharacter(
                            "not in shift2 c40/text",
                            ch,
                        ))
                    }
                }
                shift = 0;
            } else {
                match ch {
                    ch @ 0..=31 => {
                        let text = map_shift3[ch as usize];
                        if upper_shift {
                            out.push(text + 128);
                            upper_shift = false;
                        } else {
                            out.push(text);
                        }
                    }
                    _ => {
                        return Err(DataDecodingError::UnexpectedCharacter(
                            "not in shift3 c40/text",
                            ch,
                        ))
                    }
                }
                shift = 0;
            }
        }
    }
    if data.peek(0) == Some(UNLATCH) {
        // single UNLATCH at end of data
        let _ = data.eat().unwrap();
    }
    Ok((data, EncodationType::Ascii))
}

#[test]
fn test_ascii() {
    let mut out = vec![];
    let mut eci = vec![];
    assert_eq!(
        decode_ascii(Reader(b"BCD\x82\xeb\x26", 0), &mut out, &mut eci),
        Ok((Reader(&[], 6), EncodationType::Ascii))
    );
    assert_eq!(&out, b"ABC00\xa5");
}

#[test]
fn test_c40() {
    assert_eq!(decode_data(&[230, 91, 11]), Ok(vec![b'A', b'I', b'M']));
}

#[test]
fn test_edifact() {
    assert_eq!(
        decode_data(&[240, 16, 21, 1]),
        Ok(vec![b'D', b'A', b'T', b'A'])
    );
}

#[test]
fn test_base256() {
    assert_eq!(
        decode_data(&[231, 44, 108, 59, 226, 126, 1, 104]),
        Ok(vec![0xab, 0xe4, 0xf6, 0xfc, 0xe9, 0xbb])
    );
}

#[test]
fn test_read_eci() {
    use crate::encodation::GenericDataEncoder;

    fn enc_dec(eci: u32) -> u32 {
        let symbols = crate::SymbolList::default();
        let mut encoder = GenericDataEncoder::with_size(&[], &symbols);
        encoder.write_eci(eci);
        let (cw, _) = encoder.codewords().unwrap();
        read_eci(Reader(&cw[1..], 0)).unwrap().1
    }

    for eci in (0..=999999).step_by(31) {
        assert_eq!(enc_dec(eci), eci);
    }
    assert_eq!(enc_dec(0), 0);
    assert_eq!(enc_dec(126), 126);
    assert_eq!(enc_dec(127), 127);
    assert_eq!(enc_dec(16382), 16382);
    assert_eq!(enc_dec(16383), 16383);
    assert_eq!(enc_dec(999999), 999999);
}
