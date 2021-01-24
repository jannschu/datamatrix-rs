use super::{DataEncodingError, EncodingContext};

pub(crate) const LATCH_C40: u8 = 230;
pub(crate) const LATCH_BASE256: u8 = 231;
pub(crate) const LATCH_X12: u8 = 238;
pub(crate) const LATCH_TEXT: u8 = 239;
pub(crate) const LATCH_EDIFACT: u8 = 240;
pub(crate) const PAD: u8 = 129;

pub(crate) const UPPER_SHIFT: u8 = 235;

pub(super) fn two_digits_coming(rest: &[u8]) -> bool {
    match rest {
        [a, b, ..] => a.is_ascii_digit() && b.is_ascii_digit(),
        _ => false,
    }
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), DataEncodingError> {
    loop {
        // If two digits are next, encode without asking for mode switch
        let two_digits = two_digits_coming(ctx.rest());
        if two_digits {
            let a = ctx.eat().unwrap();
            let b = ctx.eat().unwrap();
            ctx.push((a - b'0') * 10 + (b - b'0') + 130);
            continue;
        }
        if ctx.maybe_switch_mode(false, 0)? {
            return Ok(());
        }
        match ctx.eat() {
            None => return Ok(()),
            Some(ch @ 0..=127) => ctx.push(ch + 1),
            Some(ch @ 128..=255) => {
                ctx.push(UPPER_SHIFT);
                ctx.push(ch - 128 + 1);
            }
        }
    }
}

/// Compute the number of bytes needed to encode `rest` in Ascii mode
pub(super) fn encoding_size(mut rest: &[u8]) -> usize {
    let mut count = 0;
    loop {
        if two_digits_coming(rest) {
            count += 1;
            rest = &rest[2..];
            continue;
        }
        if let Some((ch, tail)) = rest.split_first() {
            rest = tail;
            match ch {
                0..=127 => count += 1,
                128..=255 => count += 2,
            }
        } else {
            break;
        }
    }
    count
}
