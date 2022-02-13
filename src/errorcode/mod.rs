//! Reed-Solomon error correction codes.
//!
//! The error correction in a Data Matrix is done using Reed-Solomon codes.
//!
//! Assuming you have never heard of coding theory: By putting some redundancy
//! into the Data Matrix one can recover from, say, detection or printing errors
//! when trying to read a Data Matrix. A clever way to add redundancy
//! is the Reed-Solomon code. The details are relatively
//! math heavy and involve, for example, "higher" algebra (Galois fields).
//! Any book about coding theory should cover it, for example
//! "Error Correction Coding: Mathematical Methods and Algorithms" by Moon.
//!
//! While there is only way to compute the error code (called _encoding_),
//! there are several algorithms for error correction (called _decoding_).
//!
//! The decoder implemented in this module is _syndrome_ based. Such a decoder
//! is classically made of four main steps:
//!
//! 1. Compute the syndrome values.
//! 2. Compute the error locator polynomial
//! 3. Compute the error locations.
//! 4. Compute the error values.
//!   
//! We use a Levinson-Durbin algorithm in the second step. See
//! the article ["Levinson-Durbin Algorithm Used For Fast BCH Decoding"](https://doi.org/10.1007/978-1-4615-6119-4_1)
//! by Michael Schmidt and Gerhard P. Fettweis. This approach was empiricially
//! verified to be better than a LU decomposition and it did also beat
//! the Berlekamp-Massey algorithm (about 10% faster).
//!
//! Furthermore, in step four the [Björck-Pereyra algorithm](https://doi.org/10.1090/S0025-5718-1970-0290541-1)
//! is used to determine the error values. It was faster than Forney's algorithm
//! and also faster than a naive LU decomposition in our tests.
//!
//! The other possibilites mentionend for step 2 and 4
//! are still in the source code in case someone is interested in them.
mod decoding;
mod galois;

use alloc::{vec, vec::Vec};

use super::symbol_size::SymbolSize;
use galois::GF;

pub use decoding::decode as decode_error;
pub use decoding::ErrorDecodingError;

#[cfg(test)]
use pretty_assertions::assert_eq;

/// The coefficients of the generator polynomicals used
/// by the Reed-Solomon code specified for Data Matrix.
///
/// The coefficients are given in the standard, but can also
/// be computed with the Python script "gf.py" in this repository.
const GENERATOR_POLYNOMIALS: [&[u8]; 25] = [
    // 5
    &[1, 62, 111, 15, 48, 228],
    // 7
    &[1, 254, 92, 240, 134, 144, 68, 23],
    // 10
    &[1, 61, 110, 255, 116, 248, 223, 166, 185, 24, 28],
    // 11
    &[1, 120, 97, 60, 245, 39, 168, 194, 12, 205, 138, 175],
    // 12
    &[1, 242, 100, 178, 97, 213, 142, 42, 61, 91, 158, 153, 41],
    // 14
    &[
        1, 185, 83, 186, 18, 45, 138, 119, 157, 9, 95, 252, 192, 97, 156,
    ],
    // 15
    &[
        1, 93, 223, 139, 159, 35, 145, 234, 176, 153, 88, 201, 148, 33, 88, 116,
    ],
    // 18
    &[
        1, 188, 90, 48, 225, 254, 94, 129, 109, 213, 241, 61, 66, 75, 188, 39, 100, 195, 83,
    ],
    // 20
    &[
        1, 172, 186, 174, 27, 82, 108, 79, 253, 145, 153, 160, 188, 2, 168, 71, 233, 9, 244, 195,
        15,
    ],
    // 22
    &[
        1, 236, 188, 3, 209, 217, 125, 10, 38, 152, 1, 132, 27, 94, 71, 123, 5, 241, 95, 201, 76,
        249, 75,
    ],
    // 24
    &[
        1, 193, 50, 96, 184, 181, 12, 124, 254, 172, 5, 21, 155, 223, 251, 197, 155, 21, 176, 39,
        109, 205, 88, 190, 52,
    ],
    // 27
    &[
        1, 232, 180, 161, 246, 233, 134, 72, 108, 210, 246, 244, 65, 55, 123, 29, 229, 47, 205,
        143, 74, 97, 147, 182, 96, 130, 183, 215,
    ],
    // 28
    &[
        1, 255, 93, 168, 233, 151, 120, 136, 141, 213, 110, 138, 17, 121, 249, 34, 75, 53, 170,
        151, 37, 174, 103, 96, 71, 97, 43, 231, 211,
    ],
    // 32
    &[
        1, 104, 129, 163, 234, 55, 95, 144, 174, 249, 2, 145, 104, 19, 103, 202, 211, 38, 2, 120,
        209, 58, 61, 21, 236, 95, 107, 199, 228, 130, 159, 184, 227,
    ],
    // 34
    &[
        1, 139, 4, 179, 189, 67, 14, 89, 138, 237, 152, 62, 154, 62, 107, 114, 189, 6, 143, 95, 90,
        116, 237, 103, 42, 95, 203, 234, 71, 172, 210, 218, 231, 197, 190,
    ],
    // 36
    &[
        1, 112, 81, 98, 225, 25, 59, 184, 175, 44, 115, 119, 95, 137, 101, 33, 68, 4, 2, 18, 229,
        182, 80, 251, 220, 179, 84, 120, 102, 181, 162, 250, 130, 218, 242, 127, 245,
    ],
    // 38
    &[
        1, 235, 181, 165, 241, 166, 169, 220, 128, 80, 134, 170, 223, 122, 215, 83, 183, 55, 211,
        139, 103, 172, 41, 203, 123, 143, 233, 74, 237, 168, 102, 90, 166, 222, 239, 141, 101, 30,
        109,
    ],
    // 41
    &[
        1, 149, 220, 249, 68, 38, 81, 71, 79, 244, 224, 15, 133, 132, 208, 211, 90, 165, 84, 144,
        137, 250, 156, 120, 101, 136, 172, 193, 216, 99, 53, 48, 194, 222, 6, 142, 2, 43, 106, 123,
        21, 35,
    ],
    // 42
    &[
        1, 5, 9, 5, 226, 177, 150, 50, 69, 202, 248, 101, 54, 57, 253, 1, 21, 121, 57, 111, 214,
        105, 167, 9, 100, 95, 175, 8, 242, 133, 245, 2, 122, 105, 247, 153, 22, 38, 19, 31, 137,
        193, 77,
    ],
    // 46
    &[
        1, 78, 62, 74, 235, 114, 62, 141, 178, 40, 98, 144, 118, 173, 138, 72, 43, 21, 77, 47, 127,
        130, 206, 33, 221, 83, 171, 135, 29, 11, 61, 47, 51, 111, 129, 35, 186, 232, 160, 178, 114,
        135, 113, 200, 197, 29, 195,
    ],
    // 48
    &[
        1, 19, 225, 253, 92, 213, 69, 175, 160, 147, 187, 87, 176, 44, 82, 240, 186, 138, 66, 100,
        120, 88, 131, 205, 170, 90, 37, 23, 118, 147, 16, 106, 191, 87, 237, 188, 205, 231, 238,
        133, 238, 22, 117, 32, 96, 223, 172, 132, 245,
    ],
    // 50
    &[
        1, 74, 54, 162, 91, 167, 218, 212, 183, 156, 74, 16, 153, 79, 231, 112, 28, 25, 185, 8,
        241, 243, 76, 80, 14, 205, 156, 65, 114, 251, 241, 14, 142, 9, 16, 112, 230, 36, 223, 222,
        74, 245, 123, 150, 102, 167, 43, 165, 254, 166, 1,
    ],
    // 56
    &[
        1, 46, 143, 53, 233, 107, 203, 43, 155, 28, 247, 67, 127, 245, 137, 13, 164, 207, 62, 117,
        201, 150, 22, 238, 144, 232, 29, 203, 117, 234, 218, 146, 228, 54, 132, 200, 38, 223, 36,
        159, 150, 235, 215, 192, 230, 170, 175, 29, 100, 208, 220, 17, 12, 238, 223, 9, 175,
    ],
    // 62
    &[
        1, 204, 11, 47, 86, 124, 224, 166, 94, 7, 232, 107, 4, 170, 176, 31, 163, 17, 188, 130, 40,
        10, 87, 63, 51, 218, 27, 6, 147, 44, 161, 71, 114, 64, 175, 221, 185, 106, 250, 190, 197,
        63, 245, 230, 134, 112, 185, 37, 196, 108, 143, 189, 201, 188, 202, 118, 39, 210, 144, 50,
        169, 93, 242,
    ],
    // 68
    &[
        1, 186, 82, 103, 96, 63, 132, 153, 108, 54, 64, 189, 211, 232, 49, 25, 172, 52, 59, 241,
        181, 239, 223, 136, 231, 210, 96, 232, 220, 25, 179, 167, 202, 185, 153, 139, 66, 236, 227,
        160, 15, 213, 93, 122, 68, 177, 158, 197, 234, 180, 248, 136, 213, 127, 73, 36, 154, 244,
        147, 33, 89, 56, 159, 149, 251, 89, 173, 228, 220,
    ],
];

fn generator(len: usize) -> &'static [u8] {
    GENERATOR_POLYNOMIALS
        .iter()
        .find(|p| p.len() - 1 == len)
        .expect("no generator polynomical defined for this symbol size, this is a bug")
}

/// Compute the Reed-Solomon code used by Data Matrix for error correction.
///
/// Depending on the symbol size, the data is first split up into
/// interleaved blocks. For each block an error code is computed.
/// The resulting blocks of error codes are returned interleaved.
pub fn encode_error(data: &[u8], size: SymbolSize) -> Vec<u8> {
    let setup = size.block_setup();
    let num_codewords = size.num_data_codewords();
    assert!(data.len() == num_codewords);
    let gen = generator(setup.num_ecc_per_block);
    // For bigger symbol sizes the data is split up into interleaved blocks
    // for which an error code is computed individually. we store
    // the error blocks interleaved in the returned result.
    let stride = setup.num_ecc_blocks;
    let mut ecc = vec![0; setup.num_ecc_per_block + 1];
    let mut full_ecc = vec![0; setup.num_ecc_per_block * setup.num_ecc_blocks];
    for block in 0..setup.num_ecc_blocks {
        for item in &mut ecc {
            // reset ecc for new block
            *item = 0;
        }
        let strided_data_input = (block..data.len()).step_by(stride).map(|i| data[i]);
        ecc_block(strided_data_input, gen, &mut ecc);

        // copy block interleaved to result vector
        for (result, ecc_i) in full_ecc
            .iter_mut()
            .skip(block)
            .step_by(stride)
            .zip(&ecc[..setup.num_ecc_per_block])
        {
            debug_assert_eq!(*result, 0);
            *result = *ecc_i;
        }
    }
    full_ecc
}

fn ecc_block<T: Iterator<Item = u8>>(data: T, g: &[u8], ecc: &mut [u8]) {
    // Let d be the data polynomical (n coefficients) and g the generating polynomical
    // with k + 1 coefficients.
    //
    // We use a variant of euclidean polynomical division on the input polynomials
    // d(x) * x^k and g to get a quotient q and remainder r such that
    //
    //     d(x) * x^k = q(x) g(x) + r(x).
    //
    // The error code then is -r(x) = r(x), because
    //
    //     d(x) * x^k - r(x)
    //
    // is then divisible by g. The first n bytes will contain the data, and
    // the last k the error code, i.e., the coefficient of r. The algorithm
    // is modified to not compute q and store r directly in ecc. The ecc
    // array is used to store intermediate results.
    let ecc_len = g.len() - 1;
    for a in data {
        let k = GF(ecc[0]) + GF(a);
        for j in 0..ecc_len {
            ecc[j] = (GF(ecc[j + 1]) + k * GF(g[j + 1])).into();
        }
    }
}

#[test]
fn ecc_block_1() {
    // The test case was computed with the Python script
    let data = [23, 40, 11];
    let g = GENERATOR_POLYNOMIALS[0];
    let mut ecc = vec![0; 5 + 1];
    ecc_block(data.iter().cloned(), g, &mut ecc);
    assert_eq!(ecc[..5], vec![255, 207, 37, 244, 81]);
}
