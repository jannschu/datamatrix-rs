use super::{c40, DataEncodingError, EncodingContext};
use arrayvec::ArrayVec;

fn low_ascii_to_text_symbols(ctx: &mut ArrayVec<[u8; 6]>, ch: u8) {
    let new_ch = match ch {
        // switch case
        ch @ b'A'..=b'Z' => ch - b'A' + b'a',
        ch @ b'a'..=b'z' => ch - b'a' + b'A',
        ch => ch,
    };
    c40::low_ascii_to_c40_symbols(ctx, new_ch);
}

pub fn in_base_set(ch: u8) -> bool {
    matches!(ch, b' ' | b'0'..=b'9' | b'a'..=b'z')
}

pub fn val_size(ch: u8) -> u8 {
    match ch {
        b' ' | b'0'..=b'9' | b'a'..=b'z' => 1,
        0..=31 | 33..=47 | 58..=96 | 123..=127 => 2,
        ch @ 128..=255 => 2 + val_size(ch - 128),
    }
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), DataEncodingError> {
    c40::encode_generic(ctx, low_ascii_to_text_symbols)
}
