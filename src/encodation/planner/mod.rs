//! This modules finds an optimal encodation mode.
//!
//! It is almost decoupled from the rest of the crate. You can use this
//! to decide at any point in the input to which encodation mode to switch.
mod ascii;
mod base256;
mod c40;
mod edifact;
mod text;
mod x12;

mod frac;
mod generic;
mod shortest_path;
use frac::Frac;

pub(crate) use shortest_path::optimize;

trait ContextInformation: Clone {
    fn symbol_size_left(&self, extra_chars: usize) -> Option<usize>;

    fn rest(&self) -> &[u8];

    fn eat(&mut self) -> Option<u8>;

    fn write(&mut self, bytes: usize);

    fn peek(&self, n: usize) -> Option<u8> {
        self.rest().get(n).copied()
    }

    fn characters_left(&self) -> usize {
        self.rest().len()
    }

    fn has_more_characters(&self) -> bool {
        !self.rest().is_empty()
    }
}

#[derive(Debug, PartialEq)]
struct StepResult {
    /// Signals that nothing was done, planer is at the end of input.
    end: bool,
    /// Signals that this step can not be beaten by a prior mode swithc
    unbeatable: bool,
}

trait Plan: Clone {
    type Context;

    /// Get the new cost we would get after switching to the ASCII mode (if possible).
    fn mode_switch_cost(&self) -> Option<Frac>;

    /// Get the current cost.
    fn cost(&self) -> Frac;

    /// Read the next char (if any), return None if this failed.
    fn step(&mut self) -> Option<StepResult>;

    /// Called when the mode is switched.
    fn write_unlatch(&self) -> Self::Context;
}
