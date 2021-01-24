use super::{generator, GF};

pub enum CorrectionError {
    TooManyErrors,
}

/// Correct errors in one block.
///
/// The data block consists of the data codewords and `err_len` error
/// correction codewords.
///
/// No deinterleaving is done (see error code creation). So this method
/// must be called for each block.
///
/// Actually, the whole block `dataz is the error code since Data Matrix uses
/// so called "systematic encoding" where the message is a prefix of
/// the error code.
pub fn correct_errors(data: &mut [u8], err_len: usize) -> Result<(), CorrectionError> {
    let n = data.len();
    assert!(err_len > 1, "degree of generator polynomial must be >= 1");
    assert!(
        n > err_len,
        "data length shorter than error code suffix, calling error"
    );

    // Actually, Wikipedia has a nice description of the algorithm at
    // the time of writing this, see
    //
    //    https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction#Peterson%E2%80%93Gorenstein%E2%80%93Zierler_decoder

    let mut syndromes = vec![GF(0); err_len - 1];
    let have_non_zero = primitive_element_evaluation(data, &mut syndromes);
    if !have_non_zero {
        return Ok(());
    }

    let error_locations = find_error_locations(&syndromes)?;
    todo!()
}

/// Evaluate the polynomical given by coefficients `c` at
/// x, x^2, x^3, ... and write the result to `out` in that order.
fn primitive_element_evaluation<T: Into<GF> + Copy>(c: &[T], out: &mut [GF]) -> bool {
    let mut gamma: Vec<GF> = c.iter().rev().map(|x| (*x).into()).collect();
    let mut errors = false;
    for o in out.iter_mut() {
        for (g, alpha) in gamma.iter_mut().zip(GF::primitive_powers()) {
            *g *= alpha;
        }
        *o = gamma.iter().fold(GF(0), |a, b| a + *b);
        errors = errors || (*o != GF(0));
    }
    errors
}

fn find_error_locations(syndomes: &[GF]) -> Result<Vec<u8>, CorrectionError> {
    // step 1: determine the coefficients of the error locator polynomial \Lambda_\nu
    let v = syndomes.len() / 2;
    let mut matrix = vec![GF(0); v * v]; // row major order
    let mut rhs = vec![GF(0); v];
    for i in 0..v {
        for j in 0..v {
            matrix[i * v + j] = syndomes[i + j];
        }
        rhs[i] = -syndomes[v + i];
    }

    // step 2: find the zeros of the error locator polynomial

    todo!()
}

/// Solve the linear system `matrix` * x = `b` for x.
///
/// The matrix must be square.
///
/// Returns true if a solution was found.
fn solve(mat: &mut [GF], b: &[GF], row_stride: usize) -> Option<Vec<GF>> {
    let n = b.len();
    let c = |i: usize, j: usize| i * row_stride + j;
    let mut permutation: Vec<usize> = (0..n).collect();
    for i in 0..(n - 1) {
        // find non-zero entry
        if let Some(i_nz) = (i..n).find(|k| mat[c(*k, i)] != GF(0)) {
            // swap rows
            if i_nz != i {
                permutation.swap(i, i_nz);
                for j in 0..n {
                    mat.swap(c(i, j), c(i_nz, j));
                }
            }
        } else {
            return None;
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
        return None;
    }

    // mutate b
    let mut rhs: Vec<GF> = permutation.iter().map(|per| b[*per]).collect();
    // solve Lx = rhs
    for i in 0..n {
        for j in 0..i {
            let rhs_j = rhs[j];
            rhs[i] -= mat[c(i, j)] * rhs_j;
        }
    }
    // solve Ux = rhs
    for i in (0..n).rev() {
        for j in i + 1..n {
            let rhs_j = rhs[j];
            rhs[i] -= mat[c(i, j)] * rhs_j;
        }
        rhs[i] /= mat[c(i, i)];
    }
    Some(rhs)
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
    let b = &[GF(88)];
    let x = solve(&mut mat, &b[..], 1);
    assert_eq!(x, Some(vec![GF(170)]));
}

#[test]
fn test_solve_2x2() {
    let mut mat = vec![GF(2), GF(1), GF(5), GF(2)];
    let b = &[GF(56), GF(23)];
    let x = solve(&mut mat, &b[..], 2).unwrap();
    assert_eq!(GF(2) * x[0] + GF(1) * x[1], b[0]);
    assert_eq!(GF(5) * x[0] + GF(2) * x[1], b[1]);
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
    let b = &[GF(126), GF(23), GF(99)];
    let x = solve(&mut mat, &b[..], 3).unwrap();
    assert_eq!(GF(0) * x[0] + GF(0) * x[1] + GF(8) * x[2], b[0]);
    assert_eq!(GF(89) * x[0] + GF(0) * x[1] + GF(2) * x[2], b[1]);
    assert_eq!(GF(45) * x[0] + GF(10) * x[1] + GF(5) * x[2], b[2]);
}

#[test]
fn test_solve_2x2_singular() {
    let mut mat = vec![GF(2), GF(1), GF(4), GF(2)];
    let b = &[GF(56), GF(23)];
    let x = solve(&mut mat, &b[..], 2);
    assert_eq!(x, None);
}
