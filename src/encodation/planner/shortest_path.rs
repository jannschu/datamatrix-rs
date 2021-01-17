use super::Plan;
use crate::{encodation::encodation_type::EncodationType, symbol_size::Size};

use super::generic::GenericPlan;

/// Find an optimal encodation mode to continue in (can be same).
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
pub(crate) fn optimize<S: Size>(
    data: &[u8],
    written: usize,
    mode: EncodationType,
    free_unlatch: bool,
    size: S,
    base256_written: usize,
) -> Option<EncodationType> {
    let start_plan =
        GenericPlan::for_mode(mode, data, written, size, free_unlatch, base256_written);

    let mut plans = Vec::with_capacity(36);
    let mut new_plan = Vec::with_capacity(36);

    plans.push(start_plan);

    let mut first_iteration = true;
    loop {
        let mut at_end = false;
        let use_as_start = first_iteration;
        first_iteration = false;
        for mut plan in plans.drain(0..) {
            let plan_copy_before_step = plan.clone();
            let result = if let Some(result) = plan.step() {
                result
            } else {
                // remove plan, it can not process input
                continue;
            };
            new_plan.push(plan);

            // we then add mode switches to all other modes, unless
            // the step was optimal (unbeatable) or we are at the end.
            if !result.unbeatable && !result.end {
                // this also calls step() one time.
                plan_copy_before_step.add_switches(&mut new_plan, use_as_start);
            }
            if result.end {
                // since all modes step one character at a time,
                // we can set this for all modes
                at_end = true;
            }
        }

        remove_hopeless_cases(&mut new_plan);

        if new_plan.is_empty() {
            return None;
        }

        if let Some(best_choice) = have_winner(&new_plan) {
            return Some(best_choice);
        }

        if at_end {
            // all plans are at the end of data, pick the best one,
            // they are sorted by `remove_hopeless_cases`
            return Some(new_plan[0].start_mode());
        }
        std::mem::swap(&mut plans, &mut new_plan);
    }
}

// Only keep one minimizer for every start mode.
fn remove_hopeless_cases<'a, S: Size>(list: &mut Vec<GenericPlan<'a, S>>) {
    list.sort_by_key(|a| a.cost().unwrap());
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
                let second_cost = second.cost().unwrap();
                if first_cost < second_cost {
                    list.remove(i - removed);
                    removed += 1;
                }
            } else {
                uncomparable = true;
            }
        }
        if uncomparable {
            start += 1;
        } else {
            break;
        }
    }
}

fn have_winner<'a, S: Size>(list: &[GenericPlan<'a, S>]) -> Option<EncodationType> {
    // the list now contains at most one plan for each encodation type
    match list {
        [m] => Some(m.start_mode()),
        _ => None,
    }
}

#[test]
fn test_hopeless_remove_duplicates() {
    use crate::SymbolSize;
    let mut a = GenericPlan::for_mode(
        EncodationType::Ascii,
        &[1, 2, 3],
        0,
        SymbolSize::Min,
        false,
        0,
    );
    a.step(); // cost = 1
    let mut b = GenericPlan::for_mode(
        EncodationType::Ascii,
        &[1, 2, 3],
        0,
        SymbolSize::Min,
        false,
        0,
    );
    b.step();
    b.step(); // cost = 2
    let mut c = GenericPlan::for_mode(
        EncodationType::C40,
        &[b'A', b'C', b'D'],
        0,
        SymbolSize::Min,
        false,
        0,
    );
    c.step();
    let mut list = vec![a.clone(), b.clone(), c.clone()];
    remove_hopeless_cases(&mut list);
    assert_eq!(list, vec![c, a, b]);
}

#[test]
fn test_hopeless_remove_1() {
    use crate::SymbolSize;
    let a = GenericPlan::for_mode(
        EncodationType::Ascii,
        &[1, 2, 3],
        0,
        SymbolSize::Min,
        false,
        0,
    );
    let mut b = GenericPlan::for_mode(
        EncodationType::C40,
        &[b'A', b'C', b'D'],
        0,
        SymbolSize::Min,
        false,
        0,
    );
    b.step();
    b.step();
    b.step();
    let mut list = vec![a.clone(), b];
    remove_hopeless_cases(&mut list);
    assert_eq!(list, vec![a]);
}

#[test]
fn test_hopeless_remove_2() {
    use crate::SymbolSize;
    let mut a = GenericPlan::for_mode(
        EncodationType::Ascii,
        &[1, 2, 3],
        0,
        SymbolSize::Min,
        false,
        0,
    );
    a.step();
    a.step();
    let mut c = GenericPlan::for_mode(
        EncodationType::C40,
        b"ABCDEFGH",
        0,
        SymbolSize::Min,
        false,
        0,
    );
    c.step(); // not a boundary, will not compare, so kept
    let mut list = vec![a.clone(), c.clone()];
    remove_hopeless_cases(&mut list);
    assert_eq!(list, vec![c, a]);
}

#[test]
fn test_c40_case1() {
    use crate::SymbolSize::Min;
    // C40: <latch> + DEA + BC1 + 23F + ascii('G') = 8
    // EDIFACT: <latch> + DEAB + C123 UNLATCH G =  9
    // X12: <latch> + DEA + BC1 + 23F + ascii('G) = 8
    let result = optimize(b"DEAbC123FG", 0, EncodationType::Ascii, false, Min, 0);
    assert_eq!(result, Some(EncodationType::C40));
}

#[test]
fn test_ascii_case1() {
    use crate::SymbolSize::Min;

    let result = optimize(b"ab*de", 0, EncodationType::Ascii, false, Min, 0);
    assert_eq!(result, Some(EncodationType::Ascii));
}

#[test]
fn test_x12_case1() {
    use crate::SymbolSize::Min;
    // from b"ABC>ABC123>ABCDE", which should switches to X12 until end
    let result = optimize(b"BCDE", 0, EncodationType::X12, false, Min, 0);
    assert_eq!(result, Some(EncodationType::X12));
}

#[test]
fn test_x12_case2() {
    use crate::SymbolSize::Min;
    let result = optimize(b"CP0*", 3, EncodationType::X12, false, Min, 0);
    assert_eq!(result, Some(EncodationType::X12));
}

#[test]
fn test_x12_case3() {
    use crate::SymbolSize::Min;
    // X12 Size: Latch + 3 * 2 + ascii(00) = 8
    // EDIFACT Size: Latch + 2 * 3 + UNLATCH + ascii(00) = 9
    let result = optimize(b"*********00", 0, EncodationType::Ascii, false, Min, 0);
    assert_eq!(result, Some(EncodationType::X12));
}

#[test]
fn test_edifact_case1() {
    use crate::SymbolSize::Min;
    let result = optimize(b"XX", 42, EncodationType::Edifact, false, Min, 0);
    assert_eq!(result, Some(EncodationType::Edifact));
}
