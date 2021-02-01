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

/// Encode the data as a Data Matrix ECC200.
pub fn encode(data: &[u8], symbol_size: SymbolSize) -> Result<Bitmap<bool>, DataEncodingError> {
    let (mut codewords, symbol_size) = data::encode_data(data, symbol_size)?;
    let ecc = errorcode::encode_error(&codewords, symbol_size);
    codewords.extend_from_slice(&ecc);
    let mut map = MatrixMap::new(symbol_size);
    map.traverse(&mut CodewordPlacer(codewords));
    Ok(map.bitmap())
}
