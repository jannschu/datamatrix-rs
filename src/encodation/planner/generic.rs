use alloc::{vec, vec::Vec};
use core::fmt::{Debug, Error, Formatter};

use flagset::FlagSet;

use crate::{encodation::encodation_type::EncodationType, symbol_size::SymbolList};

use super::{
    ascii::AsciiPlan, base256::Base256Plan, c40::C40Plan, edifact::EdifactPlan, frac::Frac,
    text::TextPlan, x12::X12Plan, ContextInformation, Plan, StepResult,
};

#[cfg(test)]
use pretty_assertions::assert_eq;

#[derive(Clone, PartialEq)]
pub(super) struct GenericPlan<'a> {
    extra: Frac,
    pub(super) switches: Vec<(usize, EncodationType)>,
    plan: PlanImpl<'a>,
}

impl<'a> Debug for GenericPlan<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match &self.plan {
            PlanImpl::Ascii(pl) => f.write_fmt(format_args!(
                "Ascii(start {:?}, {:?}, cw {:?}, rest {:?}, switches = {:?})",
                self.start_mode(),
                pl.cost() + self.extra,
                pl.context().written,
                pl.context().rest().len(),
                self.switches,
            )),
            PlanImpl::C40(pl) => f.write_fmt(format_args!(
                "C40(start {:?}, {:?}, cw {:?}, rest {:?}, switches = {:?})",
                self.start_mode(),
                pl.cost() + self.extra,
                pl.context().written,
                pl.context().rest().len(),
                self.switches,
            )),
            PlanImpl::Text(pl) => f.write_fmt(format_args!(
                "Text(start {:?}, {:?}, cw {:?}, rest {:?}, switches = {:?})",
                self.start_mode(),
                pl.cost() + self.extra,
                pl.context().written,
                pl.context().rest().len(),
                self.switches,
            )),
            PlanImpl::Base256(pl) => f.write_fmt(format_args!(
                "B256(start {:?}, {:?}, cw {:?}, rest {:?}, switches = {:?})",
                self.start_mode(),
                pl.cost() + self.extra,
                pl.context().written,
                pl.context().rest().len(),
                self.switches,
            )),
            PlanImpl::Edifact(pl) => f.write_fmt(format_args!(
                "EDF(start {:?}, {:?}, cw {:?}, rest {:?}, switches = {:?})",
                self.start_mode(),
                pl.cost() + self.extra,
                pl.context().written,
                pl.context().rest().len(),
                self.switches,
            )),
            PlanImpl::X12(pl) => f.write_fmt(format_args!(
                "X12(start {:?}, {:?}, cw {:?}, rest {:?}, switches = {:?})",
                self.start_mode(),
                pl.cost() + self.extra,
                pl.context().written,
                pl.context().rest().len(),
                self.switches,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlanImpl<'a> {
    Ascii(AsciiPlan<Context<'a>>),
    C40(C40Plan<Context<'a>>),
    Text(TextPlan<Context<'a>>),
    X12(X12Plan<Context<'a>>),
    Edifact(EdifactPlan<Context<'a>>),
    Base256(Base256Plan<Context<'a>>),
}

impl<'a> GenericPlan<'a> {
    /// Create an instance which starts with the given encodation type.
    ///
    /// The `free_unlatch` is currently only used by EDIFACT, it marks
    /// that 3 out of 4 values are read, a UNLATCH would now not
    /// result in additional cost.
    pub(super) fn for_mode(
        mode: EncodationType,
        data: &'a [u8],
        written: usize,
        symbol_list: &'a SymbolList,
    ) -> Self {
        let mut ctx = Context::new(data, symbol_list);
        ctx.write(written);
        let plan = match mode {
            EncodationType::Ascii => PlanImpl::Ascii(AsciiPlan::new(ctx)),
            EncodationType::C40 => PlanImpl::C40(C40Plan::new(ctx)),
            EncodationType::Text => PlanImpl::Text(TextPlan::new(ctx)),
            EncodationType::Edifact => PlanImpl::Edifact(EdifactPlan::new(ctx)),
            EncodationType::X12 => PlanImpl::X12(X12Plan::new(ctx)),
            EncodationType::Base256 => PlanImpl::Base256(Base256Plan::new(ctx)),
        };
        Self {
            extra: 0.into(),
            switches: vec![(data.len(), mode)],
            plan,
        }
    }

    /// Get the mode this plan started with.
    pub(super) fn start_mode(&self) -> EncodationType {
        self.switches[0].1
    }

    /// Get the current encodation type (mode).
    pub(super) fn current(&self) -> EncodationType {
        match self.plan {
            PlanImpl::Ascii(_) => EncodationType::Ascii,
            PlanImpl::C40(_) => EncodationType::C40,
            PlanImpl::Text(_) => EncodationType::Text,
            PlanImpl::X12(_) => EncodationType::X12,
            PlanImpl::Edifact(_) => EncodationType::Edifact,
            PlanImpl::Base256(_) => EncodationType::Base256,
        }
    }

    pub(super) fn add_switches(
        self,
        list: &mut Vec<Self>,
        rest_len: usize,
        as_start: bool,
        enabled_modes: FlagSet<EncodationType>,
    ) {
        let ascii_cost = if let Some(cost) = self.mode_switch_cost() {
            cost
        } else {
            return;
        };
        let ctx = self.write_unlatch();

        macro_rules! add_switch {
            ($plan:ident, $enum:ident, $cost_extra:expr) => {
                let switches = if as_start {
                    assert_eq!(self.switches.len(), 1);
                    vec![(rest_len, EncodationType::$enum)]
                } else {
                    let mut switches = self.switches.clone();
                    switches.push((rest_len, EncodationType::$enum));
                    switches
                };
                let mut ctx = ctx.clone();
                ctx.write($cost_extra); // LATCH byte
                let mut new = $plan::new(ctx);
                if let Some(_) = new.step() {
                    list.push(Self {
                        extra: ascii_cost + $cost_extra,
                        switches,
                        plan: PlanImpl::$enum(new),
                    });
                }
            };
        }

        // Add switch to ASCII
        if !self.is_ascii() && enabled_modes.contains(EncodationType::Ascii) {
            add_switch!(AsciiPlan, Ascii, 0);
        }

        // Add switch to Base256
        if !matches!(self.plan, PlanImpl::Base256(_))
            && enabled_modes.contains(EncodationType::Base256)
        {
            add_switch!(Base256Plan, Base256, 1);
        }

        // Add switch to Edifact
        if !matches!(self.plan, PlanImpl::Edifact(_))
            && enabled_modes.contains(EncodationType::Edifact)
        {
            add_switch!(EdifactPlan, Edifact, 1);
        }

        // Add switch to X12
        if !self.is_x12() && enabled_modes.contains(EncodationType::X12) {
            add_switch!(X12Plan, X12, 1);
        }

        // Add switch to Text
        if !matches!(self.plan, PlanImpl::Text(_)) && enabled_modes.contains(EncodationType::Text) {
            add_switch!(TextPlan, Text, 1);
        }

        // Add switch to C40
        if !self.is_c40() && enabled_modes.contains(EncodationType::C40) {
            add_switch!(C40Plan, C40, 1);
        }
    }

    fn is_ascii(&self) -> bool {
        matches!(self.plan, PlanImpl::Ascii(_))
    }

    pub(super) fn is_c40(&self) -> bool {
        matches!(self.plan, PlanImpl::C40(_))
    }

    pub(super) fn is_x12(&self) -> bool {
        matches!(self.plan, PlanImpl::X12(_))
    }

    /// Get the total cost after switching to the given mode.
    pub(super) fn cost_for_switching_to(&self, other: EncodationType) -> Option<Frac> {
        // switchting to the mode itself is free
        if self.current() == other {
            return Some(self.cost());
        }
        match (&self.plan, other) {
            // To Ascii is provided by the mode_switch_cost()
            (_, EncodationType::Ascii) => self.mode_switch_cost(),
            // For others its the leave to ASCII, and +1 if not ASCII
            (_, EncodationType::Base256) => self.mode_switch_cost().map(|x| x + 2),
            (_, _) => self.mode_switch_cost().map(|x| x + 1),
        }
    }
}

impl<'a> Plan for GenericPlan<'a> {
    type Context = Context<'a>;

    fn mode_switch_cost(&self) -> Option<Frac> {
        // only add costs from previous modes
        match &self.plan {
            PlanImpl::Ascii(pl) => pl.mode_switch_cost().map(|x| x + self.extra),
            PlanImpl::C40(pl) => pl.mode_switch_cost().map(|x| x + self.extra),
            PlanImpl::Text(pl) => pl.mode_switch_cost().map(|x| x + self.extra),
            PlanImpl::X12(pl) => pl.mode_switch_cost().map(|x| x + self.extra),
            PlanImpl::Base256(pl) => pl.mode_switch_cost().map(|x| x + self.extra),
            PlanImpl::Edifact(pl) => pl.mode_switch_cost().map(|x| x + self.extra),
        }
    }

    fn cost(&self) -> Frac {
        // only add costs from previous modes
        match &self.plan {
            PlanImpl::Ascii(pl) => pl.cost() + self.extra,
            PlanImpl::C40(pl) => pl.cost() + self.extra,
            PlanImpl::Text(pl) => pl.cost() + self.extra,
            PlanImpl::X12(pl) => pl.cost() + self.extra,
            PlanImpl::Base256(pl) => pl.cost() + self.extra,
            PlanImpl::Edifact(pl) => pl.cost() + self.extra,
        }
    }

    fn step(&mut self) -> Option<StepResult> {
        match &mut self.plan {
            PlanImpl::Ascii(pl) => pl.step(),
            PlanImpl::C40(pl) => pl.step(),
            PlanImpl::Text(pl) => pl.step(),
            PlanImpl::X12(pl) => pl.step(),
            PlanImpl::Base256(pl) => pl.step(),
            PlanImpl::Edifact(pl) => pl.step(),
        }
    }

    fn write_unlatch(&self) -> Context<'a> {
        match &self.plan {
            PlanImpl::Ascii(pl) => pl.write_unlatch(),
            PlanImpl::C40(pl) => pl.write_unlatch(),
            PlanImpl::Text(pl) => pl.write_unlatch(),
            PlanImpl::X12(pl) => pl.write_unlatch(),
            PlanImpl::Base256(pl) => pl.write_unlatch(),
            PlanImpl::Edifact(pl) => pl.write_unlatch(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Context<'a> {
    data: &'a [u8],
    symbol_list: &'a SymbolList,
    consumed: usize,
    written: usize,
}

impl<'a> Context<'a> {
    pub(super) fn new(data: &'a [u8], symbol_list: &'a SymbolList) -> Self {
        Self {
            data,
            symbol_list,
            consumed: 0,
            written: 0,
        }
    }
}

impl<'a> ContextInformation for Context<'a> {
    fn symbol_size_left(&self, extra_chars: usize) -> Option<usize> {
        let size_needed = self.written + extra_chars;
        let symbol = self.symbol_list.first_symbol_big_enough_for(size_needed)?;
        Some(symbol.num_data_codewords() - size_needed)
    }

    fn write(&mut self, size: usize) {
        self.written += size;
    }

    fn rest(&self) -> &[u8] {
        self.data
    }

    fn eat(&mut self) -> Option<u8> {
        if let Some((ch, rest)) = self.data.split_first() {
            self.data = rest;
            self.consumed += 1;
            Some(*ch)
        } else {
            None
        }
    }
}

#[test]
fn test_add_switch_ascii() {
    let symbols = crate::SymbolList::default();
    let mut plan = GenericPlan::for_mode(EncodationType::Ascii, b"[]ABC01", 0, &symbols);
    plan.step();
    plan.step();
    plan.step();
    assert_eq!(plan.cost(), 3.into());
    let mut list = vec![];
    plan.add_switches(&mut list, 20, false, EncodationType::all());
    match &list[4].plan {
        PlanImpl::C40(pl) => {
            // one char was consumed
            assert_eq!(pl.cost(), Frac::new(2, 3));
        }
        other => panic!("wrong return type, {:?}", other),
    }
    assert_eq!(list[4].cost(), Frac::new(2, 3) + 4);
}
