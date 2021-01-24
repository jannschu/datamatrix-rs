mod pgz;

use super::GF;

pub enum DecodingError {
    TooManyErrors,
    /// Error locations were found outside of the codeword.
    ///
    /// This usually means there were a lot of transmission errors, uncorrectable.
    ErrorsOutsideRange,
}

pub use pgz::decode as decode_pgz;

/// Evaluate the polynomical given by coefficients `c` at
/// x, x^2, x^3, ... and write the result to `out` in that order.
fn primitive_element_evaluation<T: Into<GF> + Copy>(c: &[T], out: &mut [GF]) -> bool {
    let mut gamma: Vec<GF> = c.iter().rev().map(|x| (*x).into()).collect();
    let mut errors = false;
    for o in out.iter_mut() {
        for (g, alpha) in gamma.iter_mut().zip(GF::primitive_powers()) {
            *g *= alpha;
        }
        *o = gamma.iter().cloned().sum();
        errors = errors || (*o != GF(0));
    }
    errors
}

/// Find the zeros of a polynomial givven by the coefficients in `c`.
fn chien_search<T: Into<GF> + Copy>(c: &[T]) -> Vec<GF> {
    let mut out = vec![];
    if c.is_empty() {
        return out;
    }
    if c.last().cloned().unwrap().into() == GF(0) {
        out.push(GF(0));
    }
    let mut gamma: Vec<GF> = c.iter().rev().map(|x| (*x).into()).collect();
    for i in 0..=255 {
        for (g, alpha) in gamma.iter_mut().zip(GF::primitive_powers()) {
            *g *= alpha;
        }
        let val: GF = gamma.iter().cloned().sum();
        if val == GF(0) {
            out.push(GF::primitive_power(i));
        }
    }
    out
}

/// Solve the linear system `matrix` * x = `b` for x.
///
/// The matrix must be square.
///
/// Returns true if a solution was found.
fn solve(mat: &mut [GF], b: &mut [GF], row_stride: usize) -> bool {
    let n = b.len();
    let c = |i: usize, j: usize| i * row_stride + j;
    for i in 0..(n - 1) {
        // find non-zero entry
        if let Some(i_nz) = (i..n).find(|k| mat[c(*k, i)] != GF(0)) {
            // swap rows
            if i_nz != i {
                b.swap(i, i_nz);
                for j in 0..n {
                    mat.swap(c(i, j), c(i_nz, j));
                }
            }
        } else {
            return false;
        };

        for k in i + 1..n {
            // compute L
            mat[c(k, i)] /= mat[c(i, i)];
            // compute U
            for j in i + 1..n {
                mat[c(k, j)] -= mat[c(k, i)] * mat[c(i, j)];
            }
        }
    }

    if mat[c(n - 1, n - 1)] == GF(0) {
        return false;
    }

    // solve Lx = b
    for i in 0..n {
        for j in 0..i {
            let b_j = b[j];
            b[i] -= mat[c(i, j)] * b_j;
        }
    }
    // solve Ux = b
    for i in (0..n).rev() {
        for j in i + 1..n {
            let b_j = b[j];
            b[i] -= mat[c(i, j)] * b_j;
        }
        b[i] /= mat[c(i, i)];
    }
    true
}

#[test]
fn test_evaluate_primitive() {
    let c = &[GF(90), GF(0), GF(23), GF(0), GF(1)];
    let mut out = vec![GF(0); 3];
    primitive_element_evaluation(c, &mut out);
    assert_eq!(out, vec![GF(100), GF(187), GF(131)]);
}

#[test]
fn test_solve_1x1() {
    let mut mat = vec![GF(5)];
    let mut b = [GF(88)];
    let solved = solve(&mut mat, &mut b[..], 1);
    assert!(solved);
    assert_eq!(b, [GF(170)]);
}

#[test]
fn test_solve_2x2() {
    let mut mat = vec![GF(2), GF(1), GF(5), GF(2)];
    let mut b = [GF(56), GF(23)];
    let solved = solve(&mut mat, &mut b[..], 2);
    assert!(solved);
    assert_eq!(GF(2) * b[0] + GF(1) * b[1], GF(56));
    assert_eq!(GF(5) * b[0] + GF(2) * b[1], GF(23));
}

#[test]
fn test_solve_3x3_permute() {
    let mut mat = vec![
        GF(0),
        GF(0),
        GF(8),
        GF(89),
        GF(0),
        GF(2),
        GF(45),
        GF(10),
        GF(5),
    ];
    let mut b = [GF(126), GF(23), GF(99)];
    let solved = solve(&mut mat, &mut b[..], 3);
    assert!(solved);
    assert_eq!(GF(0) * b[0] + GF(0) * b[1] + GF(8) * b[2], GF(126));
    assert_eq!(GF(89) * b[0] + GF(0) * b[1] + GF(2) * b[2], GF(23));
    assert_eq!(GF(45) * b[0] + GF(10) * b[1] + GF(5) * b[2], GF(99));
}

#[test]
fn test_solve_2x2_singular() {
    let mut mat = vec![GF(2), GF(1), GF(4), GF(2)];
    let mut b = [GF(56), GF(23)];
    let solved = solve(&mut mat, &mut b[..], 2);
    assert!(!solved);
}
