//! Data Matrix (ECC 200) decoding and encoding library with an optimizing encoder.
//!
//! # Usage example
//!
//! ```rust
//! # use datamatrix::SymbolSize;
//! let bitmap = datamatrix::encode(
//!     b"Hello, World!",
//!     SymbolSize::Min,
//! ).unwrap();
//! print!("{}", bitmap.unicode());
//! ```
//!
//! This toy example will print a Data Matrix using Unicode block characters.
//! For guidance on how to generate other output formats see the helper functions
//! defined for the [Bitmap struct](Bitmap), or the `examples/` directory of
//! this project.
//!
//! You can specify other symbol sizes, see [SymbolSize] for details.
//!
//! # Character encoding notes for Data Matrix
//!
//! > **TL;DR** Data should be printable ASCII because many decoders lack a proper charset
//! > handling. Latin 1 is the next best choice, otherwise you rely on auto detection hacks of
//! > decoders. This does not apply if you have control over decoding or if you are not overly paranoidal.
//!
//! This full section also applies to QR codes.
//!
//! Be careful when encoding strings which contain non printable ASCII characters.
//! While indicating for example UTF-8 encoding is possible through [ECI](https://en.wikipedia.org/wiki/Extended_Channel_Interpretation),
//! we doubt that many decoders around the world implement this.
//! Also notice that some decoders are used as a keyboard source (e.g., handheld scanners)
//! which _may_ be constrained by platform/locale specific keyboard layouts with
//! limited Unicode input capabilities. We therefore recommend to stay within
//! the _printable_ ASCII characters unless you have control over the full encoding
//! and decoding process.
//!
//! The Data Matrix specification defines ISO 8859-1 (Latin 1) as the standard
//! charset. Our tests indicate that some decoders (smartphone scanner apps) are
//! reluctant to follow this and return binary output if there are charactes in
//! the upper range, which is a safe choice. Unfortunately, some decoders try to guess the charset
//! or just always assume UTF-8.
//!
//! To forestall your question: The full 8bit range can be encoded and
//! the decoder will also return this exact input. So the problems mentioned above
//! are related to the _interpretation_ of the data and possible input limitations
//! in the case of handheld scanners.
//!
//! # Current limitations
//!
//! No visual detection is currently implemented, but the decoding backend
//! is done and exposed in the API. All that is missing is a detector to extract a matrix of true and false values
//! from an image. A general purpose detector is planned for the future, though.
//!
//! Other limitations: Currently there is no support for GS1, FCN1 characters,
//! macro characters, ECI, structured append, and
//! reader programming. The decoding output format specified in ISO/IEC 15424 is
//! also not implemented (metadata, ECI, etc.), if you have a use case for this
//! let us know.
mod decodation;
mod encodation;
pub mod errorcode;
pub mod placement;
mod symbol_size;

pub mod data;

pub use symbol_size::SymbolSize;

use encodation::DataEncodingError;
use placement::{Bitmap, MatrixMap, Visitor};

struct CodewordPlacer(Vec<u8>);

impl Visitor<bool> for CodewordPlacer {
    fn visit(&mut self, idx: usize, bits: [&mut bool; 8]) {
        let codeword = self.0[idx];
        for i in 0..8 {
            // 0 = MSB
            // 7 = LSB
            *bits[i] = ((codeword >> (7 - i)) & 1) == 1;
        }
    }
}

/// Encode data as a Data Matrix (ECC200).
///
/// Please read the [module documentation](crate) for some charset notes. If you
/// did that and your input can be represented with the Latin 1 charset you may
/// use the conversion function in the [data module](crate::data). If you only
/// use printable ASCII you can just pass the data as is.
///
/// If the data does not fit into the given size encoding will fail. The encoder
/// can automatically pick the smallest size which fits the data (see [SymbolSize])
/// but there is an upper limit.
pub fn encode(data: &[u8], symbol_size: SymbolSize) -> Result<Bitmap<bool>, DataEncodingError> {
    let (mut codewords, symbol_size) = data::encode_data(data, symbol_size, None)?;
    let ecc = errorcode::encode_error(&codewords, symbol_size);
    codewords.extend_from_slice(&ecc);
    let mut map = MatrixMap::new(symbol_size);
    map.traverse(&mut CodewordPlacer(codewords));
    Ok(map.bitmap())
}

/// Encode a string as a Data Matrix (ECC200).
#[doc(hidden)]
pub fn encode_eci(
    data: &[u8],
    symbol_size: SymbolSize,
    eci: u32,
) -> Result<Bitmap<bool>, DataEncodingError> {
    let (mut codewords, symbol_size) = data::encode_data(data, symbol_size, Some(eci))?;
    let ecc = errorcode::encode_error(&codewords, symbol_size);
    codewords.extend_from_slice(&ecc);
    let mut map = MatrixMap::new(symbol_size);
    map.traverse(&mut CodewordPlacer(codewords));
    Ok(map.bitmap())
}
