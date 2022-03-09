//! Data part de- and encoding.
//!
//! The bytes encoded into a Data Matrix symbol consist of two parts,
//! the first part is the actual information one wants to encode,
//! the second part consists of error correction bytes.
//!
//! The functions in this module can be used to de- and encode
//! the first part, the data part.
//!
//! There is no reason I can think of for an end user of the library to ever call them directly
//! but they can be useful if one needs to work on a lower level.
use alloc::{string::String, vec::Vec};
use flagset::FlagSet;

pub use crate::decodation::{decode_data, decode_str, DataDecodingError};
use crate::encodation::{planner::optimize, GenericDataEncoder};
pub use crate::encodation::{DataEncodingError, EncodationType};

use super::{SymbolList, SymbolSize};

#[cfg(test)]
use pretty_assertions::assert_eq;

/// Encode input to data codewords for Data Matrix.
pub fn encode_data(
    data: &[u8],
    symbol_list: &SymbolList,
    eci: Option<u32>,
    enabled_modes: impl Into<FlagSet<EncodationType>>,
    use_macros: bool,
) -> Result<(Vec<u8>, SymbolSize), DataEncodingError> {
    let mut encoder = GenericDataEncoder::with_size(data, symbol_list, enabled_modes.into());
    if use_macros {
        encoder.use_macro_if_possible();
    }
    if let Some(eci) = eci {
        encoder.write_eci(eci);
    }
    encoder.codewords()
}

/// Compute a plan for when to switch encodation types during data encoding.
///
/// Returns `None` if the `data` does not fit into the given `symbol_size`.
/// Otherwise the function returns a vector of tuples `(usize, EncodationType)`
/// which describe when to switch the mode. The first entry of the tuple
/// is the number of input characters left at the point of the planned mode switch.
/// For example, `(20, EncodationType::C40)` would mean that the mode shall be
/// switched to C40 when only 20 characters remain to encode.
///
/// The plan is chosen to obtain a minimal encoding size. If there are
/// multiple solutions, a plan is picked by first filtering by the "complexity"
/// of the modes, and then by the number of mode switches. If there is still
/// more than one possibility the returned plan is an implementation detail.
///
/// # Example
///
/// ```rust
/// # use datamatrix::{data::encodation_plan, EncodationType, SymbolList};
/// encodation_plan(b"Hello!", &SymbolList::default(), EncodationType::all());
/// encodation_plan(b"Hello!", &SymbolList::default(), EncodationType::Ascii | EncodationType::Edifact);
/// ```
pub fn encodation_plan(
    data: &[u8],
    symbol_list: &SymbolList,
    enabled_modes: impl Into<FlagSet<EncodationType>>,
) -> Option<Vec<(usize, EncodationType)>> {
    optimize(
        data,
        0,
        EncodationType::Ascii,
        symbol_list,
        enabled_modes.into(),
    )
}

/// Try to convert an UTF-8 encoded string to Latin 1.
pub fn utf8_to_latin1(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let latin1_ch = match ch {
            ch @ ' '..='~' => ch as u8,
            '\u{00a0}' => 160,
            '¡' => 161,
            '¢' => 162,
            '£' => 163,
            '¤' => 164,
            '¥' => 165,
            '¦' => 166,
            '§' => 167,
            '¨' => 168,
            '©' => 169,
            'ª' => 170,
            '«' => 171,
            '¬' => 172,
            '\u{00AD}' => 173,
            '®' => 174,
            '¯' => 175,
            '°' => 176,
            '±' => 177,
            '²' => 178,
            '³' => 179,
            '´' => 180,
            'µ' => 181,
            '¶' => 182,
            '·' => 183,
            '¸' => 184,
            '¹' => 185,
            'º' => 186,
            '»' => 187,
            '¼' => 188,
            '½' => 189,
            '¾' => 190,
            '¿' => 191,
            'À' => 192,
            'Á' => 193,
            'Â' => 194,
            'Ã' => 195,
            'Ä' => 196,
            'Å' => 197,
            'Æ' => 198,
            'Ç' => 199,
            'È' => 200,
            'É' => 201,
            'Ê' => 202,
            'Ë' => 203,
            'Ì' => 204,
            'Í' => 205,
            'Î' => 206,
            'Ï' => 207,
            'Ð' => 208,
            'Ñ' => 209,
            'Ò' => 210,
            'Ó' => 211,
            'Ô' => 212,
            'Õ' => 213,
            'Ö' => 214,
            '×' => 215,
            'Ø' => 216,
            'Ù' => 217,
            'Ú' => 218,
            'Û' => 219,
            'Ü' => 220,
            'Ý' => 221,
            'Þ' => 222,
            'ß' => 223,
            'à' => 224,
            'á' => 225,
            'â' => 226,
            'ã' => 227,
            'ä' => 228,
            'å' => 229,
            'æ' => 230,
            'ç' => 231,
            'è' => 232,
            'é' => 233,
            'ê' => 234,
            'ë' => 235,
            'ì' => 236,
            'í' => 237,
            'î' => 238,
            'ï' => 239,
            'ð' => 240,
            'ñ' => 241,
            'ò' => 242,
            'ó' => 243,
            'ô' => 244,
            'õ' => 245,
            'ö' => 246,
            '÷' => 247,
            'ø' => 248,
            'ù' => 249,
            'ú' => 250,
            'û' => 251,
            'ü' => 252,
            'ý' => 253,
            'þ' => 254,
            'ÿ' => 255,
            _ => return None,
        };
        out.push(latin1_ch);
    }
    Some(out)
}

/// Try to convert a Latin 1 encoded string to an UTF-8 string.
///
/// Fails if the input is contains invalid latin 1 characters.
pub fn latin1_to_utf8(latin1: &[u8]) -> Option<String> {
    let mut out = String::with_capacity(latin1.len());
    latin1_to_utf8_mut(latin1, &mut out)?;
    Some(out)
}

pub(crate) fn latin1_to_utf8_mut(latin1: &[u8], out: &mut String) -> Option<()> {
    for ch in latin1.iter().copied() {
        let utf_ch = match ch {
            ch @ b' '..=b'~' => ch as char,
            160 => '\u{00a0}',
            161 => '¡',
            162 => '¢',
            163 => '£',
            164 => '¤',
            165 => '¥',
            166 => '¦',
            167 => '§',
            168 => '¨',
            169 => '©',
            170 => 'ª',
            171 => '«',
            172 => '¬',
            173 => '\u{00AD}',
            174 => '®',
            175 => '¯',
            176 => '°',
            177 => '±',
            178 => '²',
            179 => '³',
            180 => '´',
            181 => 'µ',
            182 => '¶',
            183 => '·',
            184 => '¸',
            185 => '¹',
            186 => 'º',
            187 => '»',
            188 => '¼',
            189 => '½',
            190 => '¾',
            191 => '¿',
            192 => 'À',
            193 => 'Á',
            194 => 'Â',
            195 => 'Ã',
            196 => 'Ä',
            197 => 'Å',
            198 => 'Æ',
            199 => 'Ç',
            200 => 'È',
            201 => 'É',
            202 => 'Ê',
            203 => 'Ë',
            204 => 'Ì',
            205 => 'Í',
            206 => 'Î',
            207 => 'Ï',
            208 => 'Ð',
            209 => 'Ñ',
            210 => 'Ò',
            211 => 'Ó',
            212 => 'Ô',
            213 => 'Õ',
            214 => 'Ö',
            215 => '×',
            216 => 'Ø',
            217 => 'Ù',
            218 => 'Ú',
            219 => 'Û',
            220 => 'Ü',
            221 => 'Ý',
            222 => 'Þ',
            223 => 'ß',
            224 => 'à',
            225 => 'á',
            226 => 'â',
            227 => 'ã',
            228 => 'ä',
            229 => 'å',
            230 => 'æ',
            231 => 'ç',
            232 => 'è',
            233 => 'é',
            234 => 'ê',
            235 => 'ë',
            236 => 'ì',
            237 => 'í',
            238 => 'î',
            239 => 'ï',
            240 => 'ð',
            241 => 'ñ',
            242 => 'ò',
            243 => 'ó',
            244 => 'ô',
            245 => 'õ',
            246 => 'ö',
            247 => '÷',
            248 => 'ø',
            249 => 'ù',
            250 => 'ú',
            251 => 'û',
            252 => 'ü',
            253 => 'ý',
            254 => 'þ',
            255 => 'ÿ',
            _ => return None,
        };
        out.push(utf_ch);
    }
    Some(())
}

#[test]
fn test_macro() {
    use crate::encodation::{ascii::PAD, MACRO05, MACRO06};
    use alloc::vec;

    assert_eq!(
        encode_data(
            b"[)>\x1E05\x1D01\x1E\x04",
            &SymbolList::default(),
            None,
            EncodationType::all(),
            true,
        )
        .unwrap()
        .0,
        vec![MACRO05, 130 + 1, PAD],
    );
    assert_eq!(
        encode_data(
            b"[)>\x1E06\x1D11\x1E\x04",
            &SymbolList::default(),
            None,
            EncodationType::all(),
            true,
        )
        .unwrap()
        .0,
        vec![MACRO06, 130 + 11, PAD],
    );
}
