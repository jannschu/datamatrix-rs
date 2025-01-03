use flagset::FlagSet;

use super::Plan;
use crate::{encodation::encodation_type::EncodationType, symbol_size::SymbolList};

use super::generic::GenericPlan;

#[cfg(test)]
use alloc::vec;
use alloc::vec::Vec;

#[cfg(test)]
use pretty_assertions::assert_eq;

/// Find an optimal encodation plan.
///
/// # Arguments
///
/// - The current rest of the input characters are given in `data`.
/// - In `written` the number of codewords (length of encoding) generated so far is given.
/// - `mode` is the currently active encodation mode.
/// - `free_unlatch` is only used when `mode` is EDIFACT (state: three values written),
/// - `size` is the symbol size
/// - `base256_written` is only used when `mode` is Base256, it contains the
///   number data bytes written so far.
pub(crate) fn optimize(
    data: &[u8],
    written: usize,
    mode: EncodationType,
    symbol_list: &SymbolList,
    enabled_modes: FlagSet<EncodationType>,
) -> Option<Vec<(usize, EncodationType)>> {
    let start_plan = GenericPlan::for_mode(mode, data, written, symbol_list);

    let mut plans = Vec::with_capacity(36);
    let mut new_plan = Vec::with_capacity(36);

    if enabled_modes.contains(mode) {
        plans.push(start_plan);
    } else {
        start_plan.add_switches(&mut plans, data.len(), true, enabled_modes);
    }

    for iteration in 0usize.. {
        let mut at_end = false;
        let use_as_start = iteration == 0;

        let rest_chars = data.len() - iteration;
        for mut plan in plans.drain(0..) {
            let plan_copy_before_step = plan.clone();
            let result = if let Some(result) = plan.step() {
                result
            } else {
                plan_copy_before_step.add_switches(
                    &mut new_plan,
                    rest_chars, // chars left
                    use_as_start,
                    enabled_modes,
                );
                // remove plan, it can not process input
                continue;
            };
            new_plan.push(plan);

            // we then add mode switches to all other modes, unless
            // the step was optimal (unbeatable) or we are at the end.
            if !result.unbeatable && !result.end {
                // this also calls step() one time.
                plan_copy_before_step.add_switches(
                    &mut new_plan,
                    rest_chars,
                    use_as_start,
                    enabled_modes,
                );
            }
            if result.end {
                // since all modes step one character at a time,
                // we can set this for all modes
                at_end = true;
            }
            assert_eq!(result.end, at_end);
        }

        remove_hopeless_cases(&mut new_plan);

        if new_plan.is_empty() {
            return None;
        }

        if at_end {
            // all plans are at the end of data, pick the best one
            let mut plan = new_plan
                .into_iter()
                .min_by_key(|p| {
                    // To decide a tie we use the ordering given by ".index()"
                    let max_enc = p.switches.iter().map(|e| e.1.index()).max().unwrap();
                    (p.cost().ceil(), max_enc, p.switches.len())
                })
                .unwrap();
            plan.switches.push((0, plan.current()));

            // Remove a "switch" to ASCII if we are at the very beginning
            if written == 0 && plan.switches[0] == (data.len(), EncodationType::Ascii) {
                plan.switches.remove(0);
            }

            return Some(plan.switches);
        }
        core::mem::swap(&mut plans, &mut new_plan);
    }
    unreachable!()
}

// Only keep one minimizer for every start mode.
fn remove_hopeless_cases(list: &mut Vec<GenericPlan>) {
    list.sort_unstable_by_key(Plan::cost);

    // only keep min among all plans with tuple (start mode, current mode)
    let mut seen = [false; 6 * 6];
    let mut removed = 0;
    for i in 0..list.len() {
        let pl = &list[i - removed];
        let pl_idx = pl.start_mode().index() * 6 + pl.current().index();
        if seen[pl_idx] {
            list.remove(i - removed);
            removed += 1;
        } else {
            seen[pl_idx] = true;
        }
    }

    let mut start = 0;
    while start + 1 < list.len() {
        let first = list[start].clone();
        // Let's say `first` has current mode A.
        // If the cost of `first` switching to mode B is lower or equal
        // to another plan with current mode B, then we can remove the other plan.
        let mut removed = 0;
        let mut uncomparable = false;
        for i in start + 1..list.len() {
            let second = &list[i - removed];
            if let Some(first_cost) = first.cost_for_switching_to(second.current()) {
                let second_cost = second.cost();
                if first_cost < second_cost {
                    list.remove(i - removed);
                    removed += 1;
                }
            } else {
                uncomparable = true;
                break;
            }
        }
        if uncomparable {
            start += 1;
        } else {
            break;
        }
    }
}

#[test]
fn test_hopeless_remove_duplicates() {
    let symbols = crate::SymbolList::default();
    let mut a = GenericPlan::for_mode(EncodationType::Ascii, &[1, 2, 3], 0, &symbols);
    a.step(); // cost = 1
    let mut b = GenericPlan::for_mode(EncodationType::C40, b"ACD", 0, &symbols);
    b.step();
    b.step(); // cost = 4/3
    let mut c = GenericPlan::for_mode(EncodationType::X12, b"ACD", 0, &symbols);
    c.step();
    c.step(); // cost = 4/3
    let mut list = vec![a.clone(), b.clone(), c.clone()];
    remove_hopeless_cases(&mut list);
    assert_eq!(list, vec![a, b, c]);
}

#[test]
fn test_hopeless_remove_1() {
    let symbols = crate::SymbolList::default();
    let a = GenericPlan::for_mode(EncodationType::Ascii, &[1, 2, 3], 0, &symbols);
    let mut b = GenericPlan::for_mode(EncodationType::C40, b"ACD", 0, &symbols);
    b.step();
    b.step();
    b.step();
    let mut list = vec![a.clone(), b];
    remove_hopeless_cases(&mut list);
    assert_eq!(list, vec![a]);
}

#[test]
fn test_hopeless_remove_2() {
    let symbols = SymbolList::default();
    let mut a = GenericPlan::for_mode(EncodationType::Ascii, &[1, 2, 3], 0, &symbols);
    a.step();
    a.step();
    let mut c = GenericPlan::for_mode(EncodationType::C40, b"ABCDEFGH", 0, &symbols);
    c.step(); // not a boundary, will not compare, so kept
    let mut list = vec![a.clone(), c.clone()];
    remove_hopeless_cases(&mut list);
    assert_eq!(list, vec![c, a]);
}

#[test]
fn test_ascii_case1() {
    let result = optimize(
        b"ab*de",
        0,
        EncodationType::Ascii,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert_eq!(result.map(|v| v[0].1), Some(EncodationType::Ascii));
}

#[test]
fn test_x12_case1() {
    // from b"ABC>ABC123>ABCDE", which should switches to X12 until end
    let result = optimize(
        b"BCDE",
        0,
        EncodationType::X12,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert_eq!(result.map(|v| v[0].1), Some(EncodationType::X12));
}

#[test]
fn test_x12_case2() {
    let result = optimize(
        b"CP0*",
        3,
        EncodationType::X12,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert_eq!(result.map(|v| v[0].1), Some(EncodationType::X12));
}

#[test]
fn test_x12_case3() {
    // X12 Size: Latch + 3 * 2 + ascii(00) = 8
    // EDIFACT Size: Latch + 2 * 3 + UNLATCH + ascii(00) = 9
    let result = optimize(
        b"*********00",
        0,
        EncodationType::Ascii,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert_eq!(result.map(|v| v[0].1), Some(EncodationType::X12));
}

#[test]
fn test_edifact_case1() {
    let result = optimize(
        b"XX",
        42,
        EncodationType::Edifact,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert_eq!(result.map(|v| v[0].1), Some(EncodationType::Edifact));
}

#[test]
fn test_edifact_case2() {
    // Next char is not encodable for edifact
    let result = optimize(
        &[140, 77, 37, 91, 75, 91, 89, 91],
        971,
        EncodationType::Edifact,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert!(result.is_some());
}

#[test]
fn test_x12_case4() {
    let result = optimize(
        b"AIMaimaimaim",
        11,
        EncodationType::X12,
        &SymbolList::default(),
        EncodationType::all(),
    );
    assert_eq!(result.map(|v| v[0].1), Some(EncodationType::X12));
}
