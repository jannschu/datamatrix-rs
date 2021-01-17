use std::fmt::{Debug, Error, Formatter};

use crate::{encodation::encodation_type::EncodationType, symbol_size::Size};

use super::{
    ascii::AsciiPlan, base256::Base256Plan, c40::C40Plan, edifact::EdifactPlan, frac::Frac,
    text::TextPlan, x12::X12Plan, ContextInformation, Plan, StepResult,
};

#[derive(Clone, PartialEq)]
pub(super) struct GenericPlan<'a, S: Size> {
    start: EncodationType,
    extra: Frac,
    plan: PlanImpl<'a, S>,
}

impl<'a, S: Size> Debug for GenericPlan<'a, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match &self.plan {
            PlanImpl::Ascii(pl) => f.write_fmt(format_args!(
                "Ascii(start {:?}, {:?} + {:?}, rest {:?}, written {})",
                self.start,
                pl.cost(),
                self.extra,
                std::str::from_utf8(pl.context().rest()),
                pl.context().written,
            )),
            PlanImpl::C40(pl) => f.write_fmt(format_args!(
                "C40(start {:?}, {:?} + {:?}, rest {:?}, written {})",
                self.start,
                pl.cost(),
                self.extra,
                std::str::from_utf8(pl.context().rest()),
                pl.context().written,
            )),
            PlanImpl::Text(pl) => f.write_fmt(format_args!(
                "Text(start {:?}, {:?} + {:?}, rest {:?}, written {})",
                self.start,
                pl.cost(),
                self.extra,
                std::str::from_utf8(pl.context().rest()),
                pl.context().written,
            )),
            PlanImpl::Base256(pl) => f.write_fmt(format_args!(
                "B256(start {:?}, {:?} + {:?}, rest {:?}, written {})",
                self.start,
                pl.cost(),
                self.extra,
                std::str::from_utf8(pl.context().rest()),
                pl.context().written,
            )),
            PlanImpl::Edifact(pl) => f.write_fmt(format_args!(
                "EDF(start {:?}, {:?} + {:?}, rest {:?}, written {})",
                self.start,
                pl.cost(),
                self.extra,
                std::str::from_utf8(pl.context().rest()),
                pl.context().written,
            )),
            PlanImpl::X12(pl) => f.write_fmt(format_args!(
                "X12(start {:?}, {:?} + {:?}, rest {:?}, written {})",
                self.start,
                pl.cost(),
                self.extra,
                std::str::from_utf8(pl.context().rest()),
                pl.context().written,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlanImpl<'a, S: Size> {
    Ascii(AsciiPlan<Context<'a, S>>),
    C40(C40Plan<Context<'a, S>>),
    Text(TextPlan<Context<'a, S>>),
    X12(X12Plan<Context<'a, S>>),
    Edifact(EdifactPlan<Context<'a, S>>),
    Base256(Base256Plan<Context<'a, S>>),
}

impl<'a, S: Size> GenericPlan<'a, S> {
    /// Create an instance which starts with the given encodation type.
    ///
    /// The `free_unlatch` is currently only used by EDIFACT, it marks
    /// that 3 out of 4 values are read, a UNLATCH would now not
    /// result in additional cost.
    pub(super) fn for_mode(
        mode: EncodationType,
        data: &'a [u8],
        written: usize,
        size: S,
        free_unlatch: bool,
        base256_written: usize,
    ) -> Self {
        let mut ctx = Context::new(data, size);
        ctx.write(written);
        let plan = match mode {
            EncodationType::Ascii => PlanImpl::Ascii(AsciiPlan::new(ctx)),
            EncodationType::C40 => PlanImpl::C40(C40Plan::new(ctx)),
            EncodationType::Text => PlanImpl::Text(TextPlan::new(ctx)),
            EncodationType::Edifact => {
                PlanImpl::Edifact(EdifactPlan::with_free_unlatch(ctx, free_unlatch))
            }
            EncodationType::X12 => PlanImpl::X12(X12Plan::new(ctx)),
            EncodationType::Base256 => {
                PlanImpl::Base256(Base256Plan::with_written(ctx, base256_written))
            }
        };
        Self {
            start: mode,
            extra: 0.into(),
            plan,
        }
    }

    /// Get the mode this plan started with.
    pub(super) fn start_mode(&self) -> EncodationType {
        self.start
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

    pub(super) fn add_switches(self, list: &mut Vec<Self>, as_start: bool) {
        let ascii_cost = if let Some(cost) = self.mode_switch_cost() {
            cost
        } else {
            return;
        };
        let ctx = self.write_unlatch();

        macro_rules! add_switch {
            ($plan:ident, $enum:ident, $cost_extra:expr) => {
                let mut ctx = ctx.clone();
                ctx.write($cost_extra);
                let mut new = $plan::new(ctx);
                if let Some(_) = new.step() {
                    list.push(Self {
                        extra: ascii_cost + $cost_extra,
                        start: if as_start {
                            EncodationType::$enum
                        } else {
                            self.start
                        },
                        plan: PlanImpl::$enum(new),
                    });
                }
            };
        }

        // This order also specifies the preference for ties

        // Add switch to ASCII
        if !self.is_ascii() {
            add_switch!(AsciiPlan, Ascii, 0);
        }

        // Add switch to Base256
        if !matches!(self.plan, PlanImpl::Base256(_)) {
            add_switch!(Base256Plan, Base256, 1);
        }

        // Add switch to Edifact
        if !matches!(self.plan, PlanImpl::Edifact(_)) {
            add_switch!(EdifactPlan, Edifact, 1);
        }

        // Add switch to X12
        if !self.is_x12() {
            add_switch!(X12Plan, X12, 1);
        }

        // We put these last because their decoding is more complicated.
        // A decoder/encoder bug is more likely here

        // Add switch to Text
        if !matches!(self.plan, PlanImpl::Text(_)) {
            add_switch!(TextPlan, Text, 1);
        }

        // Add switch to C40
        if !self.is_c40() {
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
            return self.cost();
        }
        match (&self.plan, other) {
            // From Ascii it is always 1 extra
            (PlanImpl::Ascii(pl), _) => pl.cost().map(|x| x + 1),
            // To Ascii is provided by the mode_switch_cost()
            (_, EncodationType::Ascii) => self.mode_switch_cost(),

            // For others its the leave to ASCII, and +1 if not ASCII
            (_, _) => self.mode_switch_cost().map(|x| x + 1),
        }
    }

    // pub(super) fn context(&self) -> &Context<'a, S> {
    //     match &self.plan {
    //         PlanImpl::Ascii(pl) => pl.context(),
    //         PlanImpl::C40(pl) => pl.context(),
    //         PlanImpl::Text(pl) => pl.context(),
    //         PlanImpl::X12(pl) => pl.context(),
    //         PlanImpl::Base256(pl) => pl.context(),
    //         PlanImpl::Edifact(pl) => pl.context(),
    //     }
    // }
}

impl<'a, S: Size> Plan for GenericPlan<'a, S> {
    type Context = Context<'a, S>;

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

    fn cost(&self) -> Option<Frac> {
        // only add costs from previous modes
        match &self.plan {
            PlanImpl::Ascii(pl) => pl.cost().map(|x| x + self.extra),
            PlanImpl::C40(pl) => pl.cost().map(|x| x + self.extra),
            PlanImpl::Text(pl) => pl.cost().map(|x| x + self.extra),
            PlanImpl::X12(pl) => pl.cost().map(|x| x + self.extra),
            PlanImpl::Base256(pl) => pl.cost().map(|x| x + self.extra),
            PlanImpl::Edifact(pl) => pl.cost().map(|x| x + self.extra),
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

    fn write_unlatch(&self) -> Context<'a, S> {
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
pub(super) struct Context<'a, S: Size> {
    data: &'a [u8],
    size: S,
    consumed: usize,
    written: usize,
}

impl<'a, S: Size> Context<'a, S> {
    pub(super) fn new(data: &'a [u8], size: S) -> Self {
        Self {
            data,
            size,
            consumed: 0,
            written: 0,
        }
    }
}

impl<'a, S: Size> ContextInformation for Context<'a, S> {
    fn symbol_size_left(&self, extra_chars: usize) -> Option<usize> {
        let size_needed = self.written + extra_chars;
        let symbol = self.size.symbol_for(size_needed)?;
        Some(symbol.num_data_codewords().unwrap() - size_needed)
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
    use crate::SymbolSize;
    let mut plan = GenericPlan::for_mode(
        EncodationType::Ascii,
        b"[]ABC01",
        0,
        SymbolSize::Min,
        false,
        0,
    );
    plan.step();
    plan.step();
    plan.step();
    assert_eq!(plan.cost(), Some(3.into()));
    let mut list = vec![];
    plan.add_switches(&mut list, false);
    match &list[4].plan {
        PlanImpl::C40(pl) => {
            // one char was consumed
            assert_eq!(pl.cost(), Some(Frac::new(2, 3)));
        }
        other => panic!("wrong return type, {:?}", other),
    }
    assert_eq!(list[4].cost(), Some(Frac::new(2, 3) + 4));
}
