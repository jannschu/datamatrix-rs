//! Data part de- and encoding
//!
//! The bytes encoded into a DataMatrix symbol consist of two parts,
//! the first part are actual information one wants to encode,
//! the second part error correction bytes.
//!
//! The functions in this module can be used to de- and encode
//! the first part, the data part.
//!
//! There is no reason I can think for an end user of the library to ever call them directly
//! but they can be useful if one needs to work on a lower level.
pub use crate::decodation::{decode_data, DataDecodingError};
use crate::encodation::{planner::optimize, GenericDataEncoder};
pub use crate::encodation::{DataEncodingError, EncodationType};

use super::SymbolSize;

/// Encode input to data codewords for DataMatrix.
pub fn encode_data(
    data: &[u8],
    symbol_size: SymbolSize,
) -> Result<(Vec<u8>, SymbolSize), DataEncodingError> {
    let mut encoder = GenericDataEncoder::with_size(data, symbol_size);
    let cw = encoder.codewords()?;
    Ok((cw, encoder.symbol_size))
}

/// Compute a plan for when to switch encodation types during data encoding.
///
/// Returns `None` if the `data` does not fit into the given `symbol_size`.
/// Otherwise the function returns a vector of tuples `(usize, EncodationType)`
/// which describe when to switch the mode. The first entry of the tuple
/// is the number of input characters left at the point of the planned mode switch.
/// For example, `(20, EncodationType::C40)` would mean that the mode shall be
/// switched to C40 when only 20 characters remain to encode.
///
/// The plan is chosen to obtain a minimal encoding size. If there are
/// multiple solutions, a plan is picked by first filtering by the "complexity"
/// of the modes, and then by the number of mode switches. If there are still
/// more than one possibilites, the plan returned is an implementation detail.
pub fn encodation_plan(
    data: &[u8],
    symbol_size: SymbolSize,
) -> Option<Vec<(usize, EncodationType)>> {
    optimize(data, 0, EncodationType::Ascii, symbol_size)
}
