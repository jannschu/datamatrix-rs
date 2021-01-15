use arrayvec::ArrayVec;

use super::{c40, encodation_type::EncodationType, EncodationError, EncodingContext};

pub(super) fn is_native_x12(ch: u8) -> bool {
    matches!(ch, 13 | 42 | 62 | 32 | b'0'..=b'9' | b'A'..=b'Z')
}

fn enc(ch: u8) -> Result<u8, EncodationError> {
    match ch {
        13 => Ok(0),
        42 => Ok(1),
        62 => Ok(2),
        b' ' => Ok(3),
        ch @ b'0'..=b'9' => Ok(ch - b'0' + 4),
        ch @ b'A'..=b'Z' => Ok(ch - b'A' + 14),
        _ => return Err(EncodationError::IllegalX12Character),
    }
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), EncodationError> {
    while ctx.characters_left() >= 3 {
        let c1 = enc(ctx.eat().unwrap())?;
        let c2 = enc(ctx.eat().unwrap())?;
        let c3 = enc(ctx.eat().unwrap())?;
        c40::write_three_values(ctx, c1, c2, c3);
        if ctx.maybe_switch_mode() {
            break;
        }
    }

    // 5.2.7.2, special case for X12 compared to C40, single space left and and one symbol
    if ctx.characters_left() == 1
        && ctx
            .symbol_size_left(1)
            .ok_or(EncodationError::NotEnoughSpace)?
            == 0
    {
        ctx.set_mode(EncodationType::Ascii);
        return Ok(());
    }

    let mut buf = ArrayVec::new();
    // only fill the buffer if we are at a "end of data and symbol" situation
    if ctx.characters_left() <= 2 {
        while let Some(ch) = ctx.eat() {
            buf.push(ch);
        }
    }
    super::c40::handle_end(ctx, 1, buf)
}
