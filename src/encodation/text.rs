use super::{c40, EncodationError, EncodingContext};
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

pub fn val_size(ch: u8) -> u8 {
    let new_ch = match ch {
        // switch case
        ch @ b'A'..=b'Z' => ch - b'A' + b'a',
        ch @ b'a'..=b'z' => ch - b'a' + b'A',
        ch => ch,
    };
    c40::val_size(new_ch)
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), EncodationError> {
    c40::encode_generic(ctx, low_ascii_to_text_symbols)
}
