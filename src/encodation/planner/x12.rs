use super::ContextInformation;
use super::{frac::C, Frac, Plan, StepResult};
use crate::encodation::ascii;
use crate::encodation::x12::is_native_x12;

#[derive(Debug, PartialEq, Clone)]
pub(super) struct X12Plan<T: ContextInformation> {
    /// Number of values not yet written
    ctx: T,
    values: usize,
    ascii_end: Option<Frac>,
    cost: Frac,
}

impl<T: ContextInformation> X12Plan<T> {
    pub(super) fn new(ctx: T) -> Self {
        Self {
            ctx,
            ascii_end: None,
            values: 0,
            cost: 0.into(),
        }
    }

    pub(super) fn context(&self) -> &T {
        &self.ctx
    }
}

impl<T: ContextInformation> Plan for X12Plan<T> {
    type Context = T;

    fn mode_switch_cost(&self) -> Option<Frac> {
        if self.values == 0 {
            Some(self.cost + 1)
        } else {
            None
        }
    }

    fn cost(&self) -> Frac {
        self.cost
    }

    fn write_unlatch(&self) -> Self::Context {
        assert_eq!(self.values, 0);
        assert!(self.ascii_end.is_none());
        let mut ctx = self.ctx.clone();
        ctx.write(1);
        ctx
    }

    fn step(&mut self) -> Option<StepResult> {
        let end = !self.ctx.has_more_characters();
        if !end {
            if self.values == 0 && self.ctx.characters_left() <= 2 && self.ascii_end.is_none() {
                // are we in a possible end of data situation?
                let ascii_size = ascii::encoding_size(self.ctx.rest());
                if ascii_size == 1 {
                    let space_left = self.ctx.symbol_size_left(ascii_size)?;
                    if space_left <= 1 {
                        if space_left == 1 {
                            // unlatch
                            self.cost += 1;
                        }
                        let portion_per_char =
                            Frac::new(ascii_size as C, self.ctx.characters_left() as C);
                        self.ascii_end = Some(portion_per_char);
                    }
                }
                if self.ascii_end.is_none() {
                    // there are two chars remaining, with either ascii_size > 1 or
                    // two much space left. in this scenario pure ascii is not
                    // beatable (go through the cases)
                    self.cost += 1; // unlatch
                    let portion_per_char =
                        Frac::new(ascii_size as C, self.ctx.characters_left() as C);
                    self.ascii_end = Some(portion_per_char);
                }
            }
            if self.ascii_end.is_none() && !is_native_x12(self.ctx.eat().unwrap()) {
                return None;
            }
            if let Some(portion_per_char) = self.ascii_end {
                // add (ascii_size / chars_to_read) every char read to get the correct size
                let _ = self.ctx.eat().unwrap();
                self.cost += portion_per_char;
            } else {
                self.cost += Frac::new(2, 3);
                self.values = (self.values + 1) % 3;
                if self.values == 0 {
                    self.ctx.write(2);
                }
            }
        }
        Some(StepResult {
            end,
            unbeatable: self.ascii_end.is_some(),
        })
    }
}

#[test]
fn test_eod_case1() {
    use super::generic::Context;
    use crate::SymbolSize;

    let mut plan = X12Plan::new(Context::new(b"DEABCFG", SymbolSize::Min));
    for _ in 0..7 {
        assert!(plan.step().is_some());
    }
    assert_eq!(plan.cost(), 5.into());
}

#[test]
fn test_eod_case2() {
    use super::generic::Context;
    use crate::SymbolSize;

    let mut plan = X12Plan::new(Context::new(b"AIMAIMAIMAIMAI", SymbolSize::Min));
    for i in 0..12 {
        assert!(plan.step().is_some(), "char {}", i + 1);
    }
    assert_eq!(plan.cost(), 8.into());
    // there are two chars (AI) remaining but the symbol is too large,
    // total cost 3, split ov
    assert!(plan.step().is_some());
    assert_eq!(plan.cost(), 10.into());
    assert!(plan.step().is_some());
    assert_eq!(plan.cost(), 11.into());
}
