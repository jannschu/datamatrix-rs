use super::{ascii, c40, DataEncodingError, EncodingContext};

pub(crate) fn is_native_x12(ch: u8) -> bool {
    matches!(ch, 13 | 42 | 62 | 32 | b'0'..=b'9' | b'A'..=b'Z')
}

fn enc(ch: u8) -> u8 {
    match ch {
        13 => 0,
        42 => 1,
        62 => 2,
        b' ' => 3,
        ch @ b'0'..=b'9' => ch - b'0' + 4,
        ch @ b'A'..=b'Z' => ch - b'A' + 14,
        _ => unreachable!(),
    }
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), DataEncodingError> {
    let mut switch = false;
    while ctx.characters_left() >= 3 {
        let c1 = enc(ctx.eat().unwrap());
        let c2 = enc(ctx.eat().unwrap());
        let c3 = enc(ctx.eat().unwrap());
        c40::write_three_values(ctx, c1, c2, c3);
        if ctx.maybe_switch_mode()? {
            switch = true;
            break;
        }
    }

    // 5.2.7.2, special case for X12 compared to C40, single space left and and one symbol
    let one_ascii_remain_maybe =
        ctx.characters_left() <= 2 && ascii::encoding_size(ctx.rest()) == 1;
    if one_ascii_remain_maybe
        && ctx
            .symbol_size_left(1)
            .ok_or(DataEncodingError::TooMuchOrIllegalData)?
            == 0
    {
        ctx.set_ascii_until_end();
        return Ok(());
    }
    if ctx.has_more_characters()
        || ctx
            .symbol_size_left(0)
            .ok_or(DataEncodingError::TooMuchOrIllegalData)?
            > 0
    {
        if !switch {
            ctx.set_ascii_until_end();
        }
        ctx.push(super::UNLATCH);
    }
    Ok(())
}
