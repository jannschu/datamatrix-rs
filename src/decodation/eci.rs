use alloc::string::String;
use core::iter::once;

use super::DataDecodingError;

/// Convert supported ECI sections to UTF-8.
pub fn convert(raw: &[u8], ecis: &[(usize, u32)]) -> Result<String, DataDecodingError> {
    let mut out = String::new();
    let end = (raw.len(), 0);
    let ecis = once(&(0, 0)).chain(ecis.iter()).chain(once(&end));
    for ((i, eci), j) in ecis.clone().zip(ecis.skip(1).map(|x| x.0)) {
        convert_chunk(&raw[*i..j], *eci, &mut out)?;
    }
    Ok(out)
}

fn convert_chunk(bytes: &[u8], eci: u32, out: &mut String) -> Result<(), DataDecodingError> {
    match eci {
        0 | 3 => {
            crate::data::latin1_to_utf8_mut(bytes, out).ok_or(DataDecodingError::CharsetError)?
        }
        11 => decode_iso_8859_9(bytes, out)?,
        13 => decode_iso_8859_11(bytes, out)?,
        26 => out.push_str(core::str::from_utf8(bytes).or(Err(DataDecodingError::CharsetError))?),
        27 => {
            if bytes.is_ascii() {
                out.push_str(core::str::from_utf8(bytes).or(Err(DataDecodingError::CharsetError))?);
            } else {
                return Err(DataDecodingError::CharsetError);
            }
        }
        _ => convert_chunk_extended(bytes, eci, out)?,
    }
    Ok(())
}

#[cfg(feature = "extended_eci")]
fn convert_chunk_extended(
    bytes: &[u8],
    eci: u32,
    out: &mut String,
) -> Result<(), DataDecodingError> {
    use encoding_rs::*;

    let encoder = match eci {
        4 => ISO_8859_2,
        5 => ISO_8859_3,
        6 => ISO_8859_4,
        7 => ISO_8859_5,
        8 => ISO_8859_6,
        9 => ISO_8859_7,
        10 => ISO_8859_8,
        // no support in encoding_rs as they are not allowed in HTML5
        // 11 => ISO_8859_9,
        // 13 => ISO_8859_11,
        12 => ISO_8859_10,
        15 => ISO_8859_13,
        16 => ISO_8859_14,
        17 => ISO_8859_15,
        18 => ISO_8859_16,
        20 => SHIFT_JIS,
        21 => WINDOWS_1250,
        22 => WINDOWS_1251,
        23 => WINDOWS_1252,
        24 => WINDOWS_1256,
        25 => UTF_16BE,
        // 26 => UTF-8,
        // 27 => US-ASCII,
        28 => BIG5,
        29 => GB18030,
        30 => EUC_KR,
        _ => return Err(DataDecodingError::NotImplemented("unknown ECI charset")),
    };
    let result = encoder.decode_without_bom_handling_and_without_replacement(bytes);
    out.push_str(&result.ok_or(DataDecodingError::CharsetError)?);
    Ok(())
}

#[cfg(not(feature = "extended_eci"))]
fn convert_chunk_extended(
    bytes: &[u8],
    eci: u32,
    out: &mut String,
) -> Result<(), DataDecodingError> {
    match eci {
        0..=13 | 15..=18 | 20..=30 => Err(DataDecodingError::NotImplemented(
            "compiled without support for this ECI charset, enable feature extended_eci",
        )),
        _ => Err(DataDecodingError::NotImplemented("unknown ECI charset")),
    }
}

// Source: ftp://ftp.unicode.org/Public/MAPPINGS/ISO8859/8859-11.TXT
const ISO_8859_11: [char; 88] = [
    '\u{00A0}', '\u{0E01}', '\u{0E02}', '\u{0E03}', '\u{0E04}', '\u{0E05}', '\u{0E06}', '\u{0E07}',
    '\u{0E08}', '\u{0E09}', '\u{0E0A}', '\u{0E0B}', '\u{0E0C}', '\u{0E0D}', '\u{0E0E}', '\u{0E0F}',
    '\u{0E10}', '\u{0E11}', '\u{0E12}', '\u{0E13}', '\u{0E14}', '\u{0E15}', '\u{0E16}', '\u{0E17}',
    '\u{0E18}', '\u{0E19}', '\u{0E1A}', '\u{0E1B}', '\u{0E1C}', '\u{0E1D}', '\u{0E1E}', '\u{0E1F}',
    '\u{0E20}', '\u{0E21}', '\u{0E22}', '\u{0E23}', '\u{0E24}', '\u{0E25}', '\u{0E26}', '\u{0E27}',
    '\u{0E28}', '\u{0E29}', '\u{0E2A}', '\u{0E2B}', '\u{0E2C}', '\u{0E2D}', '\u{0E2E}', '\u{0E2F}',
    '\u{0E30}', '\u{0E31}', '\u{0E32}', '\u{0E33}', '\u{0E34}', '\u{0E35}', '\u{0E36}', '\u{0E37}',
    '\u{0E38}', '\u{0E39}', '\u{0E3A}', '\u{0E3F}', '\u{0E40}', '\u{0E41}', '\u{0E42}', '\u{0E43}',
    '\u{0E44}', '\u{0E45}', '\u{0E46}', '\u{0E47}', '\u{0E48}', '\u{0E49}', '\u{0E4A}', '\u{0E4B}',
    '\u{0E4C}', '\u{0E4D}', '\u{0E4E}', '\u{0E4F}', '\u{0E50}', '\u{0E51}', '\u{0E52}', '\u{0E53}',
    '\u{0E54}', '\u{0E55}', '\u{0E56}', '\u{0E57}', '\u{0E58}', '\u{0E59}', '\u{0E5A}', '\u{0E5B}',
];

fn decode_iso_8859_11(bytes: &[u8], out: &mut String) -> Result<(), DataDecodingError> {
    for ch in bytes.iter().cloned() {
        match ch {
            0x20..=0x7E => out.push(ch as char),
            0xA0..=251 => out.push(ISO_8859_11[(ch - 128) as usize]),
            _ => return Err(DataDecodingError::CharsetError),
        }
    }
    Ok(())
}

// Source: ftp://ftp.unicode.org/Public/MAPPINGS/ISO8859/8859-9.TXT
const ISO_8859_9: [char; 96] = [
    '\u{00A0}', '\u{00A1}', '\u{00A2}', '\u{00A3}', '\u{00A4}', '\u{00A5}', '\u{00A6}', '\u{00A7}',
    '\u{00A8}', '\u{00A9}', '\u{00AA}', '\u{00AB}', '\u{00AC}', '\u{00AD}', '\u{00AE}', '\u{00AF}',
    '\u{00B0}', '\u{00B1}', '\u{00B2}', '\u{00B3}', '\u{00B4}', '\u{00B5}', '\u{00B6}', '\u{00B7}',
    '\u{00B8}', '\u{00B9}', '\u{00BA}', '\u{00BB}', '\u{00BC}', '\u{00BD}', '\u{00BE}', '\u{00BF}',
    '\u{00C0}', '\u{00C1}', '\u{00C2}', '\u{00C3}', '\u{00C4}', '\u{00C5}', '\u{00C6}', '\u{00C7}',
    '\u{00C8}', '\u{00C9}', '\u{00CA}', '\u{00CB}', '\u{00CC}', '\u{00CD}', '\u{00CE}', '\u{00CF}',
    '\u{011E}', '\u{00D1}', '\u{00D2}', '\u{00D3}', '\u{00D4}', '\u{00D5}', '\u{00D6}', '\u{00D7}',
    '\u{00D8}', '\u{00D9}', '\u{00DA}', '\u{00DB}', '\u{00DC}', '\u{0130}', '\u{015E}', '\u{00DF}',
    '\u{00E0}', '\u{00E1}', '\u{00E2}', '\u{00E3}', '\u{00E4}', '\u{00E5}', '\u{00E6}', '\u{00E7}',
    '\u{00E8}', '\u{00E9}', '\u{00EA}', '\u{00EB}', '\u{00EC}', '\u{00ED}', '\u{00EE}', '\u{00EF}',
    '\u{011F}', '\u{00F1}', '\u{00F2}', '\u{00F3}', '\u{00F4}', '\u{00F5}', '\u{00F6}', '\u{00F7}',
    '\u{00F8}', '\u{00F9}', '\u{00FA}', '\u{00FB}', '\u{00FC}', '\u{0131}', '\u{015F}', '\u{00FF}',
];

fn decode_iso_8859_9(bytes: &[u8], out: &mut String) -> Result<(), DataDecodingError> {
    for ch in bytes.iter().cloned() {
        match ch {
            0x20..=0x7E => out.push(ch as char),
            0xA0..=255 => out.push(ISO_8859_9[(ch - 128) as usize]),
            _ => return Err(DataDecodingError::CharsetError),
        }
    }
    Ok(())
}
