//! Property tests that pin the invariants between the planner and the encoder.
//!
//! The planner (`planner::optimize`) and the real encoders are two parallel
//! implementations of the same per-mode logic: the planner predicts a cost in
//! codewords, the encoder emits the bytes. These tests guard the seams between
//! them:
//!
//! 1. `planner_cost_matches_encoder_length` — the planner's predicted cost
//!    equals the number of codewords the encoder actually emits (before
//!    padding). This is the core consistency invariant; a mismatch means the
//!    duplicated end-of-data logic has drifted out of lockstep.
//! 2. `more_modes_never_increase_size` — enabling more encodation modes can
//!    only keep the symbol the same size or shrink it. A cheap optimality
//!    guard that needs no internals.
//! 3. `round_trip` — encode followed by decode returns the input (the property
//!    the AFL fuzz target also checks).

use alloc::vec::Vec;

use flagset::FlagSet;
use proptest::prelude::*;

use super::EncodationType;
use crate::data::{decode_data, encode_data, encode_data_unpadded_len};
use crate::encodation::planner::optimize_cost;
use crate::symbol_size::{SymbolList, SymbolSize};

/// All encodation modes, in declaration order so bit `i` maps to `MODES[i]`.
const MODES: [EncodationType; 6] = [
    EncodationType::Ascii,
    EncodationType::C40,
    EncodationType::Text,
    EncodationType::X12,
    EncodationType::Edifact,
    EncodationType::Base256,
];

/// Build an enabled-mode set from a bit mask. ASCII is always included since
/// every other scheme is invoked from ASCII (ISO/IEC 16022, clause 5.2.3).
fn modes_from_mask(mask: u8) -> FlagSet<EncodationType> {
    let mut set = FlagSet::from(EncodationType::Ascii);
    for (i, mode) in MODES.iter().enumerate().skip(1) {
        if mask & (1 << i) != 0 {
            set |= *mode;
        }
    }
    set
}

/// A mix of arbitrary and structured inputs that exercise the mode switches and
/// the end-of-data edge cases of each scheme.
fn data_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        proptest::collection::vec(any::<u8>(), 0..64), // arbitrary bytes
        proptest::collection::vec(0x20u8..0x7f, 0..96), // printable ASCII
        proptest::collection::vec(0x30u8..0x3a, 0..96), // digits (ASCII/C40/X12 edges)
        proptest::collection::vec(0x41u8..0x5b, 0..96), // A-Z (C40/X12 native)
        proptest::collection::vec(0x20u8..0x5f, 0..96), // EDIFACT range
    ]
}

/// Symbol lists with different shapes, including single fixed sizes whose tight
/// capacity means padding cannot mask a one-codeword divergence.
fn symbols_strategy() -> impl Strategy<Value = SymbolList> {
    prop_oneof![
        Just(SymbolList::default()),
        Just(SymbolList::with_extended_rectangles()),
        Just(SymbolList::default().enforce_square()),
        any::<prop::sample::Index>().prop_map(|idx| {
            let all: Vec<SymbolSize> = enum_iterator::all::<SymbolSize>().collect();
            SymbolList::from(all[idx.index(all.len())])
        }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    /// Whenever the encoder succeeds, the planner must predict exactly the
    /// number of codewords it emits (before padding).
    ///
    /// The reverse is not asserted: the planner can report a cost for a plan
    /// that does not fit the symbol list (ASCII planning is not bounded by
    /// symbol capacity), while the encoder correctly rejects it. The encoder
    /// is authoritative, so we only compare on the inputs it accepts.
    #[test]
    fn planner_cost_matches_encoder_length(
        data in data_strategy(),
        mask in any::<u8>(),
        symbols in symbols_strategy(),
    ) {
        let modes = modes_from_mask(mask);
        if let Some(actual) = encode_data_unpadded_len(&data, &symbols, modes) {
            let predicted = optimize_cost(&data, 0, EncodationType::Ascii, &symbols, modes);
            prop_assert_eq!(
                predicted, Some(actual),
                "planner predicted {:?} codewords but encoder produced {}; data={:?} modes={:?}",
                predicted, actual, data, modes,
            );
        }
    }

    /// Enabling more modes gives the planner strictly more options, so the
    /// chosen symbol can never grow.
    #[test]
    fn more_modes_never_increase_size(data in data_strategy(), mask in any::<u8>()) {
        let symbols = SymbolList::default();
        let size = |modes| {
            encode_data(&data, &symbols, None, modes, false)
                .ok()
                .map(|(_, symbol)| symbol.num_data_codewords())
        };
        let all = size(EncodationType::all());
        let subset = size(modes_from_mask(mask));
        if let (Some(all), Some(subset)) = (all, subset) {
            prop_assert!(
                all <= subset,
                "all modes gave {} codewords, subset {:?} gave {}; data={:?}",
                all, modes_from_mask(mask), subset, data,
            );
        }
    }

    /// Encoding and then decoding returns the original input.
    #[test]
    fn round_trip(data in data_strategy(), mask in any::<u8>()) {
        let modes = modes_from_mask(mask);
        if let Ok((codewords, _)) = encode_data(&data, &SymbolList::default(), None, modes, false) {
            let decoded = decode_data(&codewords);
            prop_assert!(decoded.is_ok(), "decode failed: {:?} for data {:?}", decoded, data);
            prop_assert_eq!(decoded.unwrap(), data);
        }
    }
}
