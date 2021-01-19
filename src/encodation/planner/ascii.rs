use super::ContextInformation;
use super::{Frac, Plan, StepResult};

#[derive(Debug, PartialEq, Clone)]
pub(super) struct AsciiPlan<T: ContextInformation> {
    /// Number of values not yet written
    ctx: T,
    digits_ahead: usize,
    cost: Frac,
}

impl<T: ContextInformation> AsciiPlan<T> {
    pub(super) fn new(ctx: T) -> Self {
        Self {
            ctx,
            digits_ahead: 0,
            cost: 0.into(),
        }
    }

    pub(super) fn context(&self) -> &T {
        &self.ctx
    }
}

impl<T: ContextInformation> Plan for AsciiPlan<T> {
    type Context = T;

    fn mode_switch_cost(&self) -> Option<Frac> {
        Some(self.cost.ceil())
    }

    fn cost(&self) -> Frac {
        self.cost
    }

    fn write_unlatch(&self) -> T {
        assert_eq!(self.digits_ahead, 0);
        self.ctx.clone()
    }

    fn step(&mut self) -> Option<StepResult> {
        // compute optimal chars, only do this when we are at a boundary and if not
        // already done
        if self.digits_ahead == 0 {
            // count number digits coming
            for ch in self.ctx.rest().iter() {
                if ch.is_ascii_digit() {
                    self.digits_ahead += 1;
                } else {
                    break;
                }
            }
            self.digits_ahead = (self.digits_ahead / 2) * 2;
            self.ctx.write(self.digits_ahead / 2);
        }
        let unbeatable = self.digits_ahead > 0;
        let end = !self.ctx.has_more_characters();
        if !end {
            let ch = self.ctx.eat().unwrap();
            if self.digits_ahead > 0 {
                self.digits_ahead -= 1;
                self.cost += Frac::new(1, 2);
            // those were already written to ctx above
            } else if ch <= 127 {
                self.cost += 1;
                self.ctx.write(1);
            } else {
                self.cost += 2;
                self.ctx.write(2);
            }
        }
        Some(StepResult { end, unbeatable })
    }
}
