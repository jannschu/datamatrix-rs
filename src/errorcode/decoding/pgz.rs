//! Implementation of the Peterson-Gorenstein-Zierler algorithm
//! for decoding Reed-Solomon codes.

use super::DecodingError;
use crate::errorcode::GF;

/// Decode the Reed-Solomon code using the PGZ algorithm .
///
/// PGZ stands for Peterson-Gorenstein-Zierler (PGZ) algorithm.
///
/// The data block `data` consists of the data codewords and `err_len` error
/// correction codewords.
///
/// No deinterleaving is done here (see error code creation). This function
/// must be called for each block.
pub fn decode(data: &mut [u8], err_len: usize) -> Result<(), DecodingError> {
    decode_gen(data, err_len, find_error_locations_lu)
}

fn decode_gen<F>(data: &mut [u8], err_len: usize, error_locs: F) -> Result<(), DecodingError>
where
    F: Fn(&[GF]) -> Result<Vec<GF>, DecodingError>,
{
    let n = data.len();
    // generator polynomial has degree d = err_len
    assert!(err_len >= 1, "degree of generator polynomial must be >= 1");
    assert!(n > err_len, "data length shorter than error code suffix");

    // Actually, Wikipedia has a nice description of the algorithm at
    // the time of writing this, see
    //
    //    https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction#Peterson%E2%80%93Gorenstein%E2%80%93Zierler_decoder

    // 1. Calculate syndromes
    let mut syndromes = vec![GF(0); err_len];
    let have_non_zero = super::primitive_element_evaluation(data, &mut syndromes);
    if !have_non_zero {
        return Ok(());
    }

    // 2. Find error locations
    let error_locations = error_locs(&syndromes)?;

    // 3. Find error values, result is computed in place in `syndromes`
    find_error_values_bp(&error_locations, &mut syndromes);

    // 4. Correct errors
    for (loc, err) in error_locations.iter().zip(syndromes.iter()) {
        let i = loc.log();
        if i >= data.len() {
            return Err(DecodingError::ErrorsOutsideRange);
        }
        let idx = data.len() - i - 1;
        data[idx] = (GF(data[idx]) - *err).into();
    }
    Ok(())
}

/// Solve the syndrome matrix equation for v,v-1,...1 using a
/// LU decomposition.
fn find_error_locations_lu(syndomes: &[GF]) -> Result<Vec<GF>, DecodingError> {
    let v = syndomes.len() / 2;

    // step 1: find the error locator polynomial

    // build syndrome matrix
    let mut matrix = vec![GF(0); v * v]; // row major order
    for i in 0..v {
        for j in 0..v {
            matrix[i * v + j] = syndomes[i + j];
        }
    }

    // try solving for decreasing v
    let mut coeff = vec![];
    for vi in (1..=v).rev() {
        let mut m = matrix.clone();
        let mut b: Vec<GF> = syndomes[vi..2 * vi].into();
        debug_assert_eq!(b.len(), vi);
        if super::solve(&mut m, &mut b[..vi], v) {
            b.truncate(vi);
            coeff = b;
            break;
        }
    }
    if coeff.is_empty() {
        // background: the syndrome matrix is regular iff. vi is equal to
        // the number of errors, since vi = 1,...,v were checked we know
        // there are too many (> v).
        return Err(DecodingError::TooManyErrors);
    }
    coeff.push(GF(1));

    // step 2: find the zeros of the error locator polynomial
    let mut zeros = super::chien_search(&coeff);

    // step 3: compute inverse of zeros to get error locations
    for z in zeros.iter_mut() {
        // z is never 0 because the constant coefficient in coeff is 1,
        // so 0 is not a zero for polynomial
        *z = GF(1) / *z;
    }
    Ok(zeros)
}

/// Find error values by solving the coefficient matrix system with the Björck-Pereyra algorithm.
///
/// This runs in O(t^2).
fn find_error_values_bp(x: &[GF], syn: &mut [GF]) {
    let e = x.len();
    // The coefficient matrix is a product of diagonal matrix and a
    // Vandermonde matrix.
    // First use the Björck-Pereyra algorithm to solve the Vandermonde system.
    for k in 0..e - 1 {
        for j in (k + 1..e).rev() {
            let tmp = syn[j - 1];
            syn[j] -= x[k] * tmp;
        }
    }
    for k in (0..e - 1).rev() {
        for j in k + 1..e {
            syn[j] /= x[j] - x[j - k - 1];
        }
        for j in k..e - 1 {
            let tmp = syn[j + 1];
            syn[j] -= tmp;
        }
    }
    // Now solve the diagonal system
    for i in 0..e {
        syn[i] /= x[i];
    }
}

#[test]
fn solve_vandermonde_diag() {
    let x = [GF(28), GF(181), GF(59), GF(129), GF(189)];
    let mut rhs = [GF(66), GF(27), GF(189), GF(255), GF(206)];
    let mut y1 = rhs.clone();
    find_error_values_bp(&x[..], &mut y1[..]);

    let mut mat = [
        GF(28),
        GF(181),
        GF(59),
        GF(129),
        GF(189),
        GF(125),
        GF(250),
        GF(220),
        GF(115),
        GF(186),
        GF(85),
        GF(18),
        GF(99),
        GF(40),
        GF(254),
        GF(66),
        GF(37),
        GF(133),
        GF(22),
        GF(175),
        GF(251),
        GF(255),
        GF(1),
        GF(36),
        GF(15),
    ];
    super::solve(&mut mat, &mut rhs, x.len());

    assert_eq!(&rhs[..], &y1);
}

#[test]
fn test_recovery() {
    let mut data = vec![1, 2, 3];
    let ecc = crate::errorcode::encode(&data, crate::SymbolSize::Square10);
    data.extend_from_slice(&ecc);
    assert_eq!(data.len(), 3 + 5);
    let mut received = data.clone();
    // make two wrong
    received[0] = 230;
    received[1] = 32;
    decode(&mut received, 5).unwrap();
    assert_eq!(&data, &received);
}
