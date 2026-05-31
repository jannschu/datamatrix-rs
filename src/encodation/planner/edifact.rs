use super::ContextInformation;
use super::{Frac, Plan, StepResult, frac::C};
use crate::encodation::ascii;
use crate::encodation::edifact::is_encodable;

#[derive(Debug, PartialEq, Clone)]
pub(super) struct EdifactPlan<T: ContextInformation> {
    /// Number of values not yet written
    ctx: T,
    written: usize,
    ascii_end: Option<Frac>,
    cost: Frac,
}

impl<T: ContextInformation> EdifactPlan<T> {
    pub(super) fn new(ctx: T) -> Self {
        Self {
            ctx,
            ascii_end: None,
            written: 0,
            cost: 0.into(),
        }
    }

    pub(super) fn context(&self) -> &T {
        &self.ctx
    }
}

impl<T: ContextInformation> Plan for EdifactPlan<T> {
    type Context = T;

    fn mode_switch_cost(&self) -> Option<Frac> {
        if self.written == 3 {
            // three of four values written, the UNLATCH is free
            Some(self.cost.ceil())
        } else {
            Some((self.cost + Frac::new(3, 4)).ceil())
        }
    }

    fn cost(&self) -> Frac {
        // The fractional estimate is exact while encoding; only the end of data
        // needs the precise flush. The ASCII-tail case (`ascii_end`) already
        // accounts for itself and never adds an UNLATCH.
        if self.ctx.has_more_characters() || self.ascii_end.is_some() {
            return self.cost;
        }
        // Mirror edifact.rs handle_end for the `written` values still buffered.
        let w = self.written;
        let space = self.ctx.symbol_size_left(w).unwrap_or(0);
        let trailing = if w == 0 {
            // Empty buffer: an UNLATCH before padding is only needed if more
            // than two codewords remain; one or two are filled with ASCII pad
            // without an UNLATCH (the EDIFACT end-of-data rule).
            if space > 2 { 1 } else { 0 }
        } else {
            // Flush the buffered values as one group, appending an UNLATCH if
            // the symbol has room or the group is full (three values).
            let symbols = if space > 0 || w == 3 { w + 1 } else { w };
            symbols.min(3)
        };
        // Replace the fractional estimate of the partial group with the exact
        // number of codewords the encoder writes for it.
        self.cost - Frac::new(3 * w as C, 4) + trailing as C
    }

    fn write_unlatch(&self) -> Self::Context {
        assert!(self.ascii_end.is_none());
        let mut ctx = self.ctx.clone();
        // the encoder will call this before any bytes are written
        ctx.write((self.written + 1).min(3));
        ctx
    }

    fn step(&mut self) -> Option<StepResult> {
        let end = !self.ctx.has_more_characters();
        if !end {
            if self.written == 0 && self.ctx.characters_left() <= 4 && self.ascii_end.is_none() {
                // are we in a possible end of data situation?
                let ascii_size = ascii::encoding_size(self.ctx.rest());
                if ascii_size <= 2 {
                    let space_left = self.ctx.symbol_size_left(ascii_size)?;
                    if space_left + ascii_size <= 2 {
                        let chars_to_read = self.ctx.characters_left();
                        self.ascii_end = Some(Frac::new(ascii_size as C, chars_to_read as C));
                    }
                }
            }
            if self.ascii_end.is_none() && !is_encodable(self.ctx.peek(0).unwrap()) {
                return None;
            }
            let _ = self.ctx.eat().unwrap();
            if let Some(portion_per_char) = self.ascii_end {
                // add (ascii_size / chars_to_read) every char read to get the correct size
                self.cost += portion_per_char;
            } else {
                self.cost += Frac::new(3, 4);
                self.written = (self.written + 1) % 4;
                if self.written == 0 {
                    self.ctx.write(3);
                }
            }
        }
        Some(StepResult {
            end,
            unbeatable: self.ascii_end.is_some(),
        })
    }
}
