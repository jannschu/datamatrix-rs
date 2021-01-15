use super::{EncodationError, EncodingContext};

pub(super) const LATCH_C40: u8 = 230;
pub(super) const LATCH_BASE256: u8 = 231;
pub(super) const LATCH_X12: u8 = 238;
pub(super) const LATCH_TEXT: u8 = 239;
pub(super) const LATCH_EDIFACT: u8 = 240;
pub(super) const PAD: u8 = 129;

const UPPER_SHIFT: u8 = 235;

fn two_digits_coming(rest: &[u8]) -> bool {
    match rest {
        [a, b, ..] => a.is_ascii_digit() && b.is_ascii_digit(),
        _ => false,
    }
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), EncodationError> {
    loop {
        let two_digits = two_digits_coming(ctx.rest());
        if two_digits {
            let a = ctx.eat().unwrap();
            let b = ctx.eat().unwrap();
            println!("push two digits {} {}", a, b);
            ctx.push((a - b'0') * 10 + (b - b'0') + 130);
        }
        if ctx.maybe_switch_mode() {
            return Ok(());
        }
        if two_digits {
            continue;
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
