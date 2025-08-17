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
    pub(super) fn with_free_unlatch(ctx: T, free_unlatch: bool) -> Self {
        Self {
            ctx,
            ascii_end: None,
            written: if free_unlatch { 3 } else { 0 },
            cost: 0.into(),
        }
    }

    pub(super) fn new(ctx: T) -> Self {
        Self::with_free_unlatch(ctx, false)
    }

    pub(super) fn context(&self) -> &T {
        &self.ctx
    }
}

impl<T: ContextInformation> Plan for EdifactPlan<T> {
    type Context = T;

    fn mode_switch_cost(&self) -> Option<Frac> {
        if self.written == 3 {
            // this also correctly handles the free_unlatch case
            Some(self.cost.ceil())
        } else {
            Some((self.cost + Frac::new(3, 4)).ceil())
        }
    }

    fn cost(&self) -> Frac {
        self.cost
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
