use arrayvec::ArrayVec;

use super::{ascii, encodation_type::EncodationType, EncodationError, EncodingContext};

const SHIFT1: u8 = 0;
const SHIFT2: u8 = 1;
const SHIFT3: u8 = 2;
const UPPER_SHIFT: u8 = 30;

#[inline(always)]
pub(super) fn low_ascii_to_c40_symbols(ctx: &mut ArrayVec<[u8; 6]>, ch: u8) {
    match ch {
        // Basic set
        b' ' => ctx.push(3),
        ch @ b'0'..=b'9' => ctx.push(ch - b'0' + 4),
        ch @ b'A'..=b'Z' => ctx.push(ch - b'A' + 14),
        // Shift 1 set
        ch @ 0..=31 => {
            ctx.push(SHIFT1);
            ctx.push(ch);
        }
        // Shift 2 set
        ch @ 33..=47 => {
            ctx.push(SHIFT2);
            ctx.push(ch - 33);
        }
        ch @ 58..=64 => {
            ctx.push(SHIFT2);
            ctx.push(ch - 58 + 15);
        }
        ch @ 91..=95 => {
            ctx.push(SHIFT2);
            ctx.push(ch - 91 + 22);
        }
        // Shift 3
        ch @ 96..=127 => {
            ctx.push(SHIFT3);
            ctx.push(ch - 96);
        }
        _ => unreachable!(),
    }
}

pub(super) fn in_base_set(ch: u8) -> bool {
    matches!(ch, b' ' | b'0'..=b'9' | b'A'..=b'Z')
}

pub(super) fn val_size(ch: u8) -> u8 {
    match ch {
        b' ' | b'0'..=b'9' | b'A'..=b'Z' => 1,
        0..=31 | 33..=47 | 58..=64 | 91..=127 => 2,
        ch => 2 + val_size(ch - 128),
    }
}

/// Encode three C40 values into two codewords.
pub(super) fn write_three_values<T: EncodingContext>(ctx: &mut T, c1: u8, c2: u8, c3: u8) {
    let enc = 1600 * c1 as u16 + 40 * c2 as u16 + c3 as u16 + 1;
    // println!("{} {} {} => {} {}", c1, c2, c3, (enc >> 8) as u8, (enc & 0xFF) as u8);
    ctx.push((enc >> 8) as u8);
    ctx.push((enc & 0xFF) as u8);
}

pub(super) fn handle_end<T>(
    ctx: &mut T,
    n_vals_last_ch: usize,
    last_ch: u8,
    mut buf: ArrayVec<[u8; 6]>,
) -> Result<(), EncodationError>
where
    T: EncodingContext,
{
    assert!(buf.len() <= 2);

    // this method is called after a requested mode switch if and only if
    // there are characters left
    let mode_switch = ctx.has_more_characters();
    if !ctx.has_more_characters() {
        let size_left = ctx
            .symbol_size_left(buf.len())
            .ok_or(EncodationError::NotEnoughSpace)?;
        match (size_left + buf.len(), buf.len()) {
            // case a) handled by standard loop
            // case b)
            (2, 2) => {
                write_three_values(ctx, buf[0], buf[1], SHIFT1);
                return Ok(());
            }
            // case c), explicit UNLATCH, rest ASCII
            (2, 1) => {
                ctx.push(super::UNLATCH);
                ctx.set_mode(EncodationType::Ascii);
                ctx.backup(1);
                return Ok(());
            }
            // case d), implicit unlatch, then ascii
            (1, 1) => {
                if ascii::encoding_size(&[last_ch]) == 1 {
                    ctx.set_mode(EncodationType::Ascii);
                    ctx.backup(1);
                    return Ok(());
                }
            }
            _ => (),
        }
    }
    if !buf.is_empty() {
        buf.push(SHIFT2);
        if buf.len() == 2 {
            buf.push(UPPER_SHIFT);
        }
        write_three_values(ctx, buf[0], buf[1], buf[2]);
        // if we were at a "end of data" situtation but there was too
        // much space for one of the cases a) - d) above, we need to explicitely
        // set the new mode, otherwise infinite loop
        if !mode_switch {
            ctx.set_mode(EncodationType::Ascii);
        }
    }
    let chars_left = ctx.characters_left();
    if chars_left > 0 {
        // exactly two remaining digits?
        if chars_left == 2 && ascii::two_digits_coming(ctx.rest()) {
            // we can encode them with one ASCII byte, maybe with UNLATCH before
            let space_left = ctx
                .symbol_size_left(1)
                .ok_or(EncodationError::NotEnoughSpace)?;
            ctx.set_mode(EncodationType::Ascii);
            if space_left >= 1 {
                ctx.push(super::UNLATCH);
            }
            return Ok(());
        }
        ctx.push(super::UNLATCH);
    } else if ctx
        .symbol_size_left(0)
        .ok_or(EncodationError::NotEnoughSpace)?
        > 0
    {
        ctx.push(super::UNLATCH);
        if !mode_switch {
            ctx.set_mode(EncodationType::Ascii);
        }
    }
    Ok(())
}

pub(super) fn encode_generic<T, F>(ctx: &mut T, low_ascii_write: F) -> Result<(), EncodationError>
where
    T: EncodingContext,
    F: Fn(&mut ArrayVec<[u8; 6]>, u8),
{
    let mut buf = ArrayVec::new();
    let mut n_vals = 0;
    let mut last_ch = 0;
    while let Some(ch) = ctx.eat() {
        // buf empty and only two digits remain?
        if buf.is_empty()
            && ch.is_ascii_digit()
            && matches!(ctx.rest(), [ch1] if ch1.is_ascii_digit())
        {
            ctx.backup(1);
            // then we can finish with:
            // - 1 codeword if 1 space is left in symbol, or with
            // - UNLATCH + 1 codeword if 2 spaces are left in the codeword
            break;
        }
        // encode the character into buf
        n_vals = to_vals(&mut buf, ch, &low_ascii_write);
        last_ch = ch;
        while buf.len() >= 3 {
            write_three_values(ctx, buf[0], buf[1], buf[2]);
            buf.drain(0..3).for_each(std::mem::drop);
        }
        if ctx.maybe_switch_mode(false, 0)? {
            break;
        }
    }
    handle_end(ctx, n_vals, last_ch, buf)
}

fn to_vals<F>(buf: &mut ArrayVec<[u8; 6]>, ch: u8, low_ascii_write: F) -> usize
where
    F: Fn(&mut ArrayVec<[u8; 6]>, u8),
{
    let len_before = buf.len();
    match ch {
        ch @ 0..=127 => low_ascii_write(buf, ch),
        ch @ 128..=255 => {
            buf.push(SHIFT2);
            buf.push(UPPER_SHIFT);
            low_ascii_write(buf, ch - 128);
        }
    };
    buf.len() - len_before
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), EncodationError> {
    encode_generic(ctx, low_ascii_to_c40_symbols)
}

#[cfg(test)]
fn vals(data: &[u8]) -> Vec<u8> {
    let mut vals = Vec::new();
    for ch in data.iter().cloned() {
        let mut buf = ArrayVec::new();
        to_vals(&mut buf, ch, low_ascii_to_c40_symbols);
        vals.extend(buf.iter());
    }
    vals
}

#[test]
fn test_enc_basic_set() {
    let vals = vals(b" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    let out: Vec<u8> = (3..=39).into_iter().collect();
    assert_eq!(vals, out);
}

#[test]
fn test_enc_shift1_set() {
    let input: Vec<u8> = (0..=31).into_iter().collect();
    let vals = vals(&input);
    assert_eq!(
        vals,
        vec![
            0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0, 9, 0, 10, 0, 11, 0, 12, 0, 13,
            0, 14, 0, 15, 0, 16, 0, 17, 0, 18, 0, 19, 0, 20, 0, 21, 0, 22, 0, 23, 0, 24, 0, 25, 0,
            26, 0, 27, 0, 28, 0, 29, 0, 30, 0, 31
        ]
    );
}

#[test]
fn test_enc_shift2_set() {
    let vals = vals(b"!\"#$%&'()*+,-./:;<=>?@[\\]^_");
    assert_eq!(
        vals,
        vec![
            1, 0, 1, 1, 1, 2, 1, 3, 1, 4, 1, 5, 1, 6, 1, 7, 1, 8, 1, 9, 1, 10, 1, 11, 1, 12, 1, 13,
            1, 14, 1, 15, 1, 16, 1, 17, 1, 18, 1, 19, 1, 20, 1, 21, 1, 22, 1, 23, 1, 24, 1, 25, 1,
            26
        ]
    );
}

#[test]
fn test_enc_shift3_set() {
    let vals = vals(b"`abcdefghijklmnopqrstuvwxyz{|}~\x7f");
    let test = vec![
        2, 0, 2, 1, 2, 2, 2, 3, 2, 4, 2, 5, 2, 6, 2, 7, 2, 8, 2, 9, 2, 10, 2, 11, 2, 12, 2, 13, 2,
        14, 2, 15, 2, 16, 2, 17, 2, 18, 2, 19, 2, 20, 2, 21, 2, 22, 2, 23, 2, 24, 2, 25, 2, 26, 2,
        27, 2, 28, 2, 29, 2, 30, 2, 31,
    ];
    assert_eq!(vals, test);
}

#[test]
fn test_shift_upper() {
    let vals = vals(b"\x80\xFF\xa0");
    // first is 1, 30, 0, 0
    // second is 1, 30, 2, 31
    // third is 1, 30, 3
    assert_eq!(vals, vec![1, 30, 0, 0, 1, 30, 2, 31, 1, 30, 3]);
}
