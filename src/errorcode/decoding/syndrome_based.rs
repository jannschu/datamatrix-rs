//! Implementation of the Peterson-Gorenstein-Zierler algorithm
//! for decoding Reed-Solomon codes.

use super::ErrorDecodingError;
use crate::errorcode::GF;
use crate::SymbolSize;

use alloc::{vec, vec::Vec};

/// Decode the Reed-Solomon code using a syndrome based decoder.
///
/// See the [module documentation](crate::errorcode) for some implementation details.
///
/// # Params
///
/// The symbol `codewords` and the `size` of the symbol. The errors are corrected
/// in-place.
///
/// For larger symbols the error codes are interleaved in a certain way
/// (see specification), this is considered in this decoder.
pub fn decode(codewords: &mut [u8], size: SymbolSize) -> Result<(), ErrorDecodingError> {
    let setup = size.block_setup();
    let err_len = setup.num_ecc_per_block;
    let stride = setup.num_ecc_blocks;
    let num_data = size.num_data_codewords();

    // For Square144 the first 8 blocks are 218 codewords (156 data codewords)
    // and the last two are 217 (155 data codewords). The stride will
    // be 10 in this case. Using just step_by(10) would give us the wrong error
    // codewords, so need to step the data and error parts separately...
    let (data, error) = codewords.split_at_mut(num_data);
    for block in 0..setup.num_ecc_blocks {
        decode_gen(
            &mut data[block..],
            &mut error[block..],
            stride,
            err_len,
            find_inv_error_locations_levinson_durbin,
            find_error_values_bp,
        )?;
    }
    Ok(())
}

fn decode_gen<F, G>(
    data: &mut [u8],
    error: &mut [u8],
    stride: usize,
    err_len: usize,
    inv_error_locs: F,
    find_err_vals: G,
) -> Result<(), ErrorDecodingError>
where
    F: Fn(&[GF]) -> Result<Vec<GF>, ErrorDecodingError>,
    G: Fn(&mut [GF], &[GF], &mut [GF]),
{
    let n_data = (data.len() + stride - 1) / stride;
    let n_error = (error.len() + stride - 1) / stride;
    let n = n_data + n_error;
    // generator polynomial has degree d = err_len
    assert!(err_len >= 1, "degree of generator polynomial must be >= 1");
    assert!(n > err_len, "data length shorter than error code suffix");

    // Actually, Wikipedia has a nice description of the (classic) algorithm at
    // the time of writing this, see
    //
    //    https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction#Peterson%E2%80%93Gorenstein%E2%80%93Zierler_decoder

    // 1. Calculate syndromes
    let mut syndromes = vec![GF(0); err_len];
    let received = data
        .iter()
        .cloned()
        .step_by(stride)
        .chain(error.iter().cloned().step_by(stride));
    let have_non_zero = super::primitive_element_evaluation(received, &mut syndromes);
    if !have_non_zero {
        return Ok(());
    }

    // 2a. Find error locations
    let lambda_coeff = inv_error_locs(&syndromes)?;
    let mut inv_error_locations = super::chien_search(&lambda_coeff);
    if inv_error_locations.len() != lambda_coeff.len() - 1 || inv_error_locations[0] == GF(0) {
        return Err(ErrorDecodingError::Malfunction);
    }

    // 2b. Check for malfunction, cf.
    // M. Srinivasan and D. V. Sarwate, Malfunction in the Peterson-Gorenstein-Zierler Decoder,
    // IEEE Trans. Inf. Theory.
    let t = err_len / 2;
    let v = lambda_coeff.len() - 1;
    for j in t..=2 * t - v - 1 {
        debug_assert!(syndromes[j..].len() >= lambda_coeff.len());
        let t_j: GF = syndromes[j..]
            .iter()
            .zip(lambda_coeff.iter())
            .map(|(a, b)| *a * *b)
            .sum();
        if t_j != GF(0) {
            return Err(ErrorDecodingError::Malfunction);
        }
    }

    // 3. Find error values, result is computed in place in `syndromes`
    find_err_vals(&mut inv_error_locations, &lambda_coeff, &mut syndromes);
    let error_locations = inv_error_locations;

    // 4. Correct errors
    for (loc, err) in error_locations.iter().zip(syndromes.iter()) {
        let i = loc.log();
        if i >= n {
            return Err(ErrorDecodingError::ErrorsOutsideRange);
        }
        let mut idx = (n - i - 1) * stride;
        if idx < data.len() {
            data[idx] = (GF(data[idx]) - *err).into();
        } else {
            idx -= data.len();
            error[idx] = (GF(error[idx]) - *err).into();
        }
    }

    Ok(())
}

/// Find the error locations by exploiting that the syndrome matrix is a Hankel matrix.
///
/// See the paper "Levinson-Durbin Algorithm Used For Fast BCH Decoding" by Schmidt and Fettweis.
fn find_inv_error_locations_levinson_durbin(syn: &[GF]) -> Result<Vec<GF>, ErrorDecodingError> {
    let t = syn.len() / 2;

    // find smallest v such that H_v is nonsingular
    let mut v = syn.iter().take_while(|s| **s == GF(0)).count() + 1;

    // initialize y = [1/b_v, 0, ..., 0]
    let mut y = Vec::with_capacity(t);
    y.push(GF(1) / syn[v - 1]);
    y.extend(core::iter::repeat(GF(0)).take(v - 1));

    // initialize w, solve lower right triangular system H_v w = h_v
    let mut w = Vec::<GF>::with_capacity(t);
    w.extend(syn[v..=2 * v - 1].iter().rev());
    for i in 0..v {
        for j in v - i..v {
            let wj = w[j];
            w[v - 1 - i] -= syn[i + j] * wj;
        }
        w[v - 1 - i] /= syn[v - 1];
    }

    fn dot(a: &[GF], b: &[GF]) -> GF {
        debug_assert_eq!(a.len(), b.len());
        a.iter().zip(b.iter()).map(|(p, q)| *p * *q).sum()
    }

    let mut tmp = Vec::with_capacity(t);
    let mut sigma = Vec::with_capacity(t);
    let mut gamma = Vec::with_capacity(t);

    while v < t {
        // Compute tmp = [w_v, - 1]
        tmp.clear();
        tmp.extend_from_slice(&w);
        tmp.push(-GF(1));

        let eps_v: GF = dot(&syn[v..=2 * v], &tmp);
        if eps_v != GF(0) {
            // "The Regular Case"

            // 1. w = [0, w], following steps are all part of eq. (6)
            w.insert(0, GF(0));

            // 2. w[..v] -= eps_v * y
            for (wi, yi) in w[..v].iter_mut().zip(y.iter()) {
                *wi -= eps_v * *yi;
            }

            let beta: GF = dot(&syn[v + 1..=2 * v + 1], &tmp) / eps_v;
            let gamma: GF = dot(&syn[v..=2 * v - 1], &y);
            // 3. w -= (beta - gamma) * eps * y_{v+1}
            for (wi, ti) in w.iter_mut().zip(tmp.iter()) {
                *wi -= (beta - gamma) * *ti;
            }

            // 4. y = [w_v, -1] / eps_v, eq. (5)
            let eps_inv = GF(1) / eps_v;
            for (yi, ti) in y.iter_mut().zip(tmp.iter()) {
                *yi = *ti * eps_inv;
            }
            y.push(-eps_inv);
            v += 1;
        } else {
            // "The Singular Case", statistically rare

            // find m, eq. (7), usually m = 1
            let m = (1..t - v)
                .filter_map(|i| {
                    let sigma_i = dot(&syn[v + i..=2 * v + i], &tmp);
                    if sigma_i != GF(0) {
                        Some((i, sigma_i))
                    } else {
                        None
                    }
                })
                .next();
            let (m, sigma_m) = if let Some((m, sigma_m)) = m {
                (m, sigma_m)
            } else {
                break;
            };
            let n = m + v;

            // compute the sigma_i used later (defined in eq. (7))
            sigma.clear();
            sigma.push(sigma_m);
            sigma.extend((m + 1..=2 * m).map(|k| dot(&syn[v + k..=2 * v + k], &tmp)));
            debug_assert_eq!(sigma.len(), m + 1);

            // Iterate w^k, store in tmp, eq. (8)
            tmp.pop(); // tmp = w_v
            for k in 0..=m {
                let rho = syn[2 * v + k] - dot(&syn[v..=2 * v - 1], &tmp);
                let eta = tmp[v - 1];
                // 1. w^k = U * w_k, shift values right
                tmp.pop();
                tmp.insert(0, GF(0));
                // 2. w^k += rho * y + eta * w_v
                for (wki, (yi, wi)) in tmp.iter_mut().zip(y.iter().zip(w.iter())) {
                    *wki += rho * *yi + eta * *wi;
                }
            }

            // update y, eq. (11)
            y.clear();
            y.resize(n + 1, GF(0));
            let sigma_m_inv = GF(1) / sigma_m;
            for (yi, w) in y.iter_mut().zip(w.iter()) {
                *yi = *w * sigma_m_inv;
            }
            y[w.len()] = -sigma_m_inv;

            // compute gamma, eq. (10)
            gamma.clear();
            gamma.extend(
                (0..=m).map(|i| syn[n + v + 1 + i] - dot(&syn[v + i..=2 * v - 1 + i], &tmp)),
            );
            for i in 0..=m {
                for j in 0..i {
                    let gj = gamma[j];
                    gamma[i] -= sigma[i - j] * gj;
                }
                gamma[i] /= sigma[0];
            }

            if cfg!(debug_assertions) {
                for i in 0..=m {
                    let mut row = GF(0);
                    for j in 0..=i {
                        row += sigma[i - j] * gamma[j]
                    }
                    let target = syn[n + v + 1 + i] - dot(&syn[v + i..=2 * v - 1 + i], &tmp);
                    debug_assert_eq!(row, target, "gamma, row {}", i)
                }
            }

            // update w, first compute result in tmp, eq. (9)
            tmp.resize(n + 1, GF(0)); // tmp = I_{n+1,v}^0 w^{m+1}_v
            for (i, gamma_i) in gamma.iter().enumerate() {
                for (ti, wj) in tmp[m - i..].iter_mut().zip(w.iter()) {
                    *ti += *gamma_i * *wj;
                }
                tmp[m - i + v] -= *gamma_i;
            }
            core::mem::swap(&mut tmp, &mut w);

            v = n + 1;
        }

        if cfg!(debug_assertions) {
            debug_assert_eq!(w.len(), v);
            debug_assert_eq!(y.len(), v);

            // check eq. (3)
            for i in 0..v {
                let mut row = GF(0);
                for j in 0..v {
                    row += syn[i + j] * y[j]
                }
                let target = if i == v - 1 { GF(1) } else { GF(0) };
                debug_assert_eq!(row, target, "y_{}, row {}", v, i)
            }
            // check eq. (4)
            for i in 0..v {
                let mut row = GF(0);
                for j in 0..v {
                    row += syn[i + j] * w[j]
                }
                debug_assert_eq!(row, syn[v + i], "w_{}, row {}", v, i);
            }
        }
    }

    w.push(GF(1));
    Ok(w)
}

/// Find error values by solving the coefficient matrix system with the Björck-Pereyra algorithm.
///
/// This runs in O(t^2).
fn find_error_values_bp(x_loc: &mut [GF], _lambda: &[GF], syn: &mut [GF]) {
    let e = x_loc.len();
    // compute inverse of zeros to get error locations
    for z in x_loc.iter_mut() {
        // z is never 0 because the constant coefficient in coeff is 1,
        // so 0 is not a zero for polynomial
        *z = GF(1) / *z;
    }
    // The coefficient matrix is a product of diagonal matrix and a
    // Vandermonde matrix.
    // First use the Björck-Pereyra algorithm to solve the Vandermonde system.
    for (k, x_loc_k) in x_loc.iter().enumerate().take(e - 1) {
        for j in (k + 1..e).rev() {
            let tmp = syn[j - 1];
            syn[j] -= *x_loc_k * tmp;
        }
    }
    for k in (0..e - 1).rev() {
        for j in k + 1..e {
            syn[j] /= x_loc[j] - x_loc[j - k - 1];
        }
        for j in k..e - 1 {
            let tmp = syn[j + 1];
            syn[j] -= tmp;
        }
    }
    // Now solve the diagonal system
    for i in 0..e {
        syn[i] /= x_loc[i];
    }
}

/// The Berlekamp-Massey (BM) algorithm for finding error locations.
#[allow(unused)]
fn find_inv_error_locations_bm(syn: &[GF]) -> Result<Vec<GF>, ErrorDecodingError> {
    let mut len_lfsr = 0; // current length of the LFSR
    let mut cur = vec![GF(1)]; // current connection polynomial
    let mut prev = vec![GF(1)]; // connection polynomial before last length change
    let mut l = 1; // l is k - m, the amount of shift in update
    let mut discrepancy_m = GF(1); // previous discrepancy
    for k in 0..syn.len() {
        // compute discrepancy
        let discrepancy = syn[k]
            + cur[1..]
                .iter()
                .zip(syn[..k].iter().rev())
                .map(|(a, b)| *a * *b)
                .sum();
        if discrepancy == GF(0) {
            l += 1;
        } else if 2 * len_lfsr > k {
            // update without length change
            let tmp = discrepancy / discrepancy_m;
            for (ci, pj) in cur[l..].iter_mut().zip(prev.iter()) {
                *ci -= tmp * *pj;
            }
            l += 1;
        } else {
            // update cur with length change
            let cur_clone_before = cur.clone();
            let tmp = discrepancy / discrepancy_m;
            cur.resize(l + prev.len(), GF(0));
            for (ci, pj) in cur[l..].iter_mut().zip(prev.iter()) {
                *ci -= tmp * *pj;
            }
            len_lfsr = k + 1 - len_lfsr;
            prev = cur_clone_before;
            discrepancy_m = discrepancy;
            l = 1;
        }
    }

    if cur.len() - 1 > syn.len() / 2 {
        Err(ErrorDecodingError::TooManyErrors)
    } else {
        cur.reverse();
        Ok(cur)
    }
}

/// Solve the syndrome matrix equation for v,v-1,...1 using a
/// LU decomposition.
#[allow(unused)]
fn find_inv_error_locations_lu(syndomes: &[GF]) -> Result<Vec<GF>, ErrorDecodingError> {
    let v = syndomes.len() / 2;

    // step 1: find the error locator polynomial

    // build syndrome matrix
    let mut matrix = vec![GF(0); v * v]; // row-major order
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
        // This method is not called if all syndromes are zero,
        // better safe than sorry => return error.
        return Err(ErrorDecodingError::TooManyErrors);
    }
    coeff.push(GF(1));
    Ok(coeff)
}

/// Find the error values using Forney's algorithm.
///
/// # Params
///
/// - `inv_x_locs` is the list the inverses of the error locations,
/// - `lambda` is the list of coefficients for the error locator polynomial (starting with highest)
/// - `syn` are the syndromes
#[allow(unused)]
fn find_error_values_forney(inv_x_locs: &mut [GF], lambda: &[GF], syn: &mut [GF]) {
    let n = syn.len();
    // compute Lambda(x) * S(x) mod x^n
    let mut omega = vec![GF(0); n];
    for (i, si) in syn.iter().cloned().enumerate() {
        // si is coefficient for x with power i
        for (j, lj) in lambda.iter().rev().take(n - i).cloned().enumerate() {
            omega[n - 1 - (j + i)] += lj * si;
        }
    }

    for (x_inv, out) in inv_x_locs.iter().cloned().zip(syn.iter_mut()) {
        let mut omega_x = GF(0);
        for o in omega.iter().cloned() {
            omega_x = omega_x * x_inv + o;
        }

        let mut lambda_der_x = GF(0);
        for (k, lk) in lambda[..lambda.len() - 1].iter().cloned().enumerate() {
            // notice that lk is multiplied with usize, this is NOT multiplication
            // in GF, see Mul<usize> implementation for GF.
            lambda_der_x = lambda_der_x * x_inv + lk * (lambda.len() - k - 1);
        }

        *out = -omega_x / lambda_der_x;
    }

    // compute inverse of x_inv to get error locations
    for z in inv_x_locs.iter_mut() {
        // z is never 0 because the constant coefficient in coeff is 1,
        // so 0 is not a zero for polynomial
        *z = GF(1) / *z;
    }
}

#[test]
fn solve_vandermonde_diag() {
    let mut x = [GF(28), GF(181), GF(59), GF(129), GF(189)];
    for tmp in x.iter_mut() {
        *tmp = GF(1) / *tmp;
    }
    let mut rhs = [GF(66), GF(27), GF(189), GF(255), GF(206)];
    let mut y1 = rhs;
    find_error_values_bp(&mut x[..], &[], &mut y1[..]);

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
    let ecc = crate::errorcode::encode_error(&data, SymbolSize::Square10);
    data.extend_from_slice(&ecc);
    assert_eq!(data.len(), 3 + 5);
    let mut received = data.clone();
    // make two wrong
    received[0] = 230;
    received[3 + 5 - 1] = 32;
    decode(&mut received, SymbolSize::Square10).unwrap();
    assert_eq!(&data, &received);
}

#[test]
fn test_recovery1() {
    let mut data = vec![
        255, 255, 255, 72, 52, 38, 52, 52, 52, 52, 52, 52, 52, 52, 52, 72, 0, 0, 72, 0, 0, 10,
    ];
    let ecc = crate::errorcode::encode_error(&data, SymbolSize::Square20);
    data.extend_from_slice(&ecc);
    let mut received = data.clone();
    received[0] = 52;
    received[8] = 144;
    decode(&mut received, SymbolSize::Square20).unwrap();
    assert_eq!(&data, &received);
}

#[test]
fn test_recovery2() {
    let mut data = vec![
        144, 144, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255,
    ];
    let ecc = crate::errorcode::encode_error(&data, SymbolSize::Square20);
    data.extend_from_slice(&ecc);
    let mut received = data.clone();
    received[1] = 32;
    received[0] = 0;
    received[12] = 0;
    received[16] = 144;
    decode(&mut received, SymbolSize::Square20).unwrap();
    assert_eq!(&data, &received);
}

#[test]
fn test_recovery3() {
    let mut data = vec![
        255, 23, 189, 54, 189, 189, 189, 189, 255, 255, 255, 255, 255, 255, 255, 67, 4, 0, 255,
        189, 48, 37,
    ];
    let ecc = crate::errorcode::encode_error(&data, SymbolSize::Square20);
    data.extend_from_slice(&ecc);
    let mut received = data.clone();
    received[0] = 247;
    received[1] = 49;
    received[5] = 255;
    received[6] = 0;
    received[8] = 49;
    received[10] = 0;
    received[12] = 65;
    received[15] = 177;
    received[16] = 32;
    decode(&mut received, SymbolSize::Square20).unwrap();
    assert_eq!(&data, &received);
}

#[test]
fn test_recovery4() {
    let mut data = vec![
        49, 95, 49, 44, 49, 49, 0, 0, 0, 32, 255, 247, 255, 254, 189, 189, 189, 189, 189, 189, 189,
        189,
    ];
    let ecc = crate::errorcode::encode_error(&data, SymbolSize::Square20);
    data.extend_from_slice(&ecc);
    let mut received = data.clone();
    received[1] = 49;
    received[0] = 44;
    received[13] = 49;
    received[10] = 0;
    received[8] = 101;
    received[15] = 54;
    received[6] = 206;
    received[21] = 191;
    received[5] = 50;
    decode(&mut received, SymbolSize::Square20).unwrap();
    assert_eq!(&data, &received);
}
