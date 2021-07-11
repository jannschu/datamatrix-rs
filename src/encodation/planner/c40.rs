use core::fmt::Debug;
use core::marker::PhantomData;

use super::frac::C;
use super::ContextInformation;
use super::{Frac, Plan, StepResult};
use crate::encodation::{ascii, c40};

pub(super) trait CharsetInfo: Clone + Debug + PartialEq {
    fn val_size(ch: u8) -> u8;

    fn in_base_set(ch: u8) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct C40Charset;

impl CharsetInfo for C40Charset {
    fn val_size(ch: u8) -> u8 {
        c40::val_size(ch)
    }

    fn in_base_set(ch: u8) -> bool {
        c40::in_base_set(ch)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct C40LikePlan<T: ContextInformation, U: CharsetInfo> {
    /// Number of values not yet written
    ctx: T,
    values: u8,
    unbeatable_reads: usize,
    ch: u8,
    two_digit_ascii_end: bool,
    cost: Frac,
    dummy: PhantomData<U>,
}

impl<T: ContextInformation, U: CharsetInfo> C40LikePlan<T, U> {
    pub(super) fn new(ctx: T) -> Self {
        Self {
            ctx,
            values: 0,
            ch: 0,
            unbeatable_reads: 0,
            cost: 0.into(),
            two_digit_ascii_end: false,
            dummy: PhantomData,
        }
    }

    pub(super) fn context(&self) -> &T {
        &self.ctx
    }
}

pub(super) fn unbeatable_strike<F>(rest: &[u8], nice_char: F) -> usize
where
    F: Fn(u8) -> bool,
{
    let mut consecutive_digits = 0;
    let mut unbeatable_reads = 0;
    for ch in rest.iter() {
        if !nice_char(*ch) {
            break;
        }
        unbeatable_reads += 1;
        // now only enough digits with ASCII can beat this
        if ch.is_ascii_digit() {
            consecutive_digits += 1;
            if consecutive_digits == 7 {
                unbeatable_reads -= consecutive_digits;
                break;
            }
        } else {
            consecutive_digits = 0;
        }
    }
    (unbeatable_reads / 3) * 3
}

impl<T: ContextInformation, U: CharsetInfo> Plan for C40LikePlan<T, U> {
    type Context = T;

    fn mode_switch_cost(&self) -> Option<Frac> {
        if self.values == 0 {
            // Are at a boundary, only one UNLATCH
            Some(self.cost + 1)
        } else {
            // Fill up, then UNLATCH
            Some(self.cost + 2 + 1)
        }
    }

    fn write_unlatch(&self) -> Self::Context {
        let mut ctx = self.ctx.clone();
        if self.values > 0 {
            assert!(self.values <= 2);
            // finish C40 pair
            ctx.write(2);
        }
        ctx.write(1);
        ctx
    }

    fn cost(&self) -> Frac {
        if self.ctx.has_more_characters() {
            return self.cost + Frac::new(2 * self.values as C, 3);
        }
        // compute additional cost to store remaining values
        let extra = if self.values == 2 {
            let space_left = self.ctx.symbol_size_left(2).unwrap_or(0);
            if space_left == 0 {
                2
            } else {
                // encode (val1, val2, 0) = 2 codewords
                // and final unlatch to continue with padding
                3
            }
        } else if self.values == 1 {
            let space_left = self.ctx.symbol_size_left(1).unwrap_or(0);
            // this also includes the `self.two_digit_ascii_end` case...
            let ascii_size = ascii::encoding_size(&[self.ch]);
            if space_left == 0 {
                if ascii_size == 1 {
                    1
                } else {
                    // we need a bigger symbol in this case (if possible)
                    1 + ascii_size
                }
            } else {
                // UNLATCH and then encode as ASCII
                1 + ascii_size
            }
        } else {
            0
        };
        self.cost + extra as C
    }

    fn step(&mut self) -> Option<StepResult> {
        // compute optimal chars, only do this when we are at a boundary and if not
        // already done
        if self.values == 0 && self.unbeatable_reads == 0 {
            // are the only remaining charactes two ascii digits?
            if matches!(self.ctx.rest(), [a, b] if a.is_ascii_digit() && b.is_ascii_digit()) {
                let space_left = self.ctx.symbol_size_left(1)?;
                self.two_digit_ascii_end = space_left <= 1;
                if space_left == 1 {
                    // UNLATCH + ASCII(two digits)
                    self.unbeatable_reads = 2;
                    self.ctx.write(2);
                } else if space_left == 0 {
                    // implicit UNLATCH, ASCII(two digits);
                    self.unbeatable_reads = 2;
                    self.ctx.write(1);
                }
            }
            if !self.two_digit_ascii_end {
                // count number of base set characters coming, watch out for digits
                self.unbeatable_reads = unbeatable_strike(self.ctx.rest(), U::in_base_set);
                self.ctx.write((self.unbeatable_reads / 3) * 2);
            }
        }
        let unbeatable = self.unbeatable_reads > 0;
        let end = !self.ctx.has_more_characters();
        if !end {
            self.ch = self.ctx.eat().unwrap();
            if self.unbeatable_reads > 0 {
                if !self.two_digit_ascii_end || self.values == 0 {
                    self.values += 1;
                }
                self.unbeatable_reads -= 1;
            } else {
                self.values += U::val_size(self.ch);
            }
            while self.values >= 3 {
                self.cost += 2;
                if !unbeatable {
                    self.ctx.write(2);
                }
                self.values -= 3;
            }
        }
        Some(StepResult { end, unbeatable })
    }
}

pub(super) type C40Plan<T> = C40LikePlan<T, C40Charset>;

#[test]
fn test_eod_case1() {
    use super::generic::Context;

    let symbols = crate::SymbolList::default();
    let mut plan = C40Plan::new(Context::new(b"DEABCFG", &symbols));
    for _ in 0..7 {
        plan.step();
    }
    assert_eq!(plan.cost(), 5.into());
}
