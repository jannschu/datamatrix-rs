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
//! You can also specify other symbol sizes, see [SymbolSize] for more details.
//!
//! # Notes
//!
//! Be careful when encoding strings which contain non-ASCII characters (Unicode values bigger than 127).
//! While support for, say, UTF-8 is possible (not implemented), be aware that
//! the implementation coverage of decoders around the world regarding this is not
//! known. Also notice that some decoders are used as a keyboard source (e.g., handheld scanners)
//! which involve platform and locale specific keyboard layouts with
//! limited Unicode input capabilities. We therefore recommend to stay within
//! the printable ASCII characters unless you have control over the full encoding
//! and decoding process.
//!
//! # Current limitations
//!
//! No visual detection is currently implemented, but the decoding backend
//! is done and exposed in the API. All that is missing is a detector to convert
//! image to a matrix of true and false values. A general purpose detector is planned for the
//! future, though.
//!
//! Other limitations: Currently there is no support for GS1, macro characters, ECI, structured append,
//! and reader programming.
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
/// If the data does not fit into the given size encoding will fail. The encoder
/// can automatically pick the smallest size which fits the data (see [SymbolSize])
/// but there is an upper limit.
pub fn encode(data: &[u8], symbol_size: SymbolSize) -> Result<Bitmap<bool>, DataEncodingError> {
    let (mut codewords, symbol_size) = data::encode_data(data, symbol_size)?;
    let ecc = errorcode::encode_error(&codewords, symbol_size);
    codewords.extend_from_slice(&ecc);
    let mut map = MatrixMap::new(symbol_size);
    map.traverse(&mut CodewordPlacer(codewords));
    Ok(map.bitmap())
}
