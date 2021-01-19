use super::ContextInformation;
use super::{Frac, Plan, StepResult};

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Base256Plan<T: ContextInformation> {
    /// Number of values not yet written
    ctx: T,
    written: usize,
    cost: Frac,
}

impl<T: ContextInformation> Base256Plan<T> {
    pub(super) fn with_written(mut ctx: T, written: usize) -> Self {
        let cost = if written == 0 {
            // for length byte
            ctx.write(1);
            1
        } else {
            0
        };
        Self {
            ctx,
            written,
            cost: cost.into(), // initial length byte
        }
    }

    pub(super) fn new(ctx: T) -> Self {
        Self::with_written(ctx, 0)
    }

    pub(super) fn context(&self) -> &T {
        &self.ctx
    }
}

impl<T: ContextInformation> Plan for Base256Plan<T> {
    type Context = T;

    fn mode_switch_cost(&self) -> Option<Frac> {
        if self.written >= 250 {
            Some(self.cost + 1)
        } else {
            Some(self.cost)
        }
    }

    fn cost(&self) -> Frac {
        if !self.ctx.has_more_characters() {
            let left = self.ctx.symbol_size_left(0).unwrap_or(1);
            if left > 0 && self.written > 249 {
                // we can must use 1 extra byte for the length
                return self.cost + 1;
            }
        }
        self.cost
    }

    fn write_unlatch(&self) -> T {
        let mut ctx = self.ctx.clone();
        if self.written >= 250 {
            // extra byte for big length
            ctx.write(1);
        }
        ctx
    }

    fn step(&mut self) -> Option<StepResult> {
        let end = !self.ctx.has_more_characters();
        if !end {
            let _ = self.ctx.eat().unwrap();
            self.written += 1;
            self.cost += 1;
            self.ctx.write(1);
            if self.written == 1556 {
                return None;
            }
        }
        Some(StepResult {
            end,
            unbeatable: false,
        })
    }
}
