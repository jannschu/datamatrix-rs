use super::{encodation_type::EncodationType, EncodationError, EncodingContext};

/// Perform the 255 state randomization as defined in the standard.
///
/// `pos` must be the number of the byte to be written w.r.t. to the full
/// codeword vector, the number is 1-based.
fn randomize_255_state(ch: u8, pos: usize) -> u8 {
    let pseudo_random = ((149 * pos) % 255) + 1;
    let tmp = ch as u16 + pseudo_random as u16;
    if tmp <= 255 {
        tmp as u8
    } else {
        (tmp - 256) as u8
    }
}

/// Write the length "header" of this encodation.
fn write_length<T: EncodingContext>(ctx: &mut T, start: usize) -> Result<(), EncodationError> {
    let space_left = ctx
        .symbol_size_left(0)
        .ok_or(EncodationError::NotEnoughSpace)?;
    let mut data_written = ctx.codewords().len() - start;
    if ctx.has_more_characters() || space_left > 0 {
        let data_count = data_written - 1;
        if data_count <= 249 {
            ctx.replace(start, data_count as u8)
        } else if data_count <= 1555 {
            ctx.replace(start, ((data_count / 250) + 249) as u8);
            ctx.insert(start + 1, (data_count % 250) as u8);
            data_written += 1;
        } else {
            // if we get here the planner has a bug
            panic!("base256 data too long, this is an encoding bug");
        }
    }
    for i in 0..data_written {
        let ch = ctx.codewords()[start + i];
        ctx.replace(start + i, randomize_255_state(ch, start + i + 1));
    }
    Ok(())
}

pub(super) fn encode<T: EncodingContext>(ctx: &mut T) -> Result<(), EncodationError> {
    let start = ctx.codewords().len();
    ctx.push(0);

    loop {
        if let Some(ch) = ctx.eat() {
            ctx.push(ch);
        }
        if !ctx.has_more_characters()
            || ctx.maybe_switch_mode(true, ctx.codewords().len() - 1 - start)?
        {
            write_length(ctx, start)?;
            if !ctx.has_more_characters() {
                ctx.set_mode(EncodationType::Ascii);
            }
            return Ok(());
        }
    }
}
