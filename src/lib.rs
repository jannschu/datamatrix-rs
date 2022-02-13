//! Data Matrix (ECC 200) decoding and encoding library with an optimizing encoder.
//!
//! # Usage example
//!
//! ```rust
//! # use datamatrix::{DataMatrix, SymbolList};
//! let code = DataMatrix::encode(
//!     b"Hello, World!",
//!     SymbolList::default(),
//! ).unwrap();
//! print!("{}", code.bitmap().unicode());
//! ```
//!
//! This toy example will print a Data Matrix using Unicode block characters.
//! For guidance on how to generate other output formats see the helper functions
//! defined for the [Bitmap struct](Bitmap), or the `examples/` directory of
//! this project.
//!
//! You can specify other symbol sizes, see [SymbolList] for details.
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
//! The Data Matrix specification defines ISO 8859-1 (Latin-1) as the standard
//! charset. Our tests indicate that some decoders (smartphone scanner apps) are
//! reluctant to follow this and return binary output if there are charactes in
//! the upper range, which is a safe choice. Unfortunately, some decoders try to guess the charset
//! or just always assume UTF-8.
//!
//! The full 8bit range can be encoded and
//! the decoder will also return this exact input. So the problems mentioned above
//! are related to the _interpretation_ of the data and possible input limitations
//! in the case of handheld scanners.
//!
//! # Decoding
//!
//! Assuming you have detected a Data Matrix you may decode the message like
//! this:
//!
//! ```rust
//! # use datamatrix::{SymbolSize, placement::MatrixMap, DataMatrix};
//! # let codewords1 = [73, 239, 116, 130, 175, 52, 19, 40, 179, 242, 106, 105, 97, 98, 35, 165, 137, 102, 203, 106, 207, 48, 186, 66];
//! # let map = MatrixMap::new_with_codewords(&codewords1, SymbolSize::Square16);
//! # let pixels: Vec<bool> = map.bitmap().bits().into();
//! // let pixels: Vec<bool> = …
//! let width = 16;
//! let data = DataMatrix::decode(&pixels, width).unwrap();
//! assert_eq!(&data, b"Hello, World!");
//! ```
//!
//! # Current limitations
//!
//! No visual detection is currently implemented, but the decoding backend
//! is done and exposed in the API. All that is missing is a detector to extract a matrix of true and false values
//! from an image. A general purpose detector is planned for the future, though.
//!
//! Other limitations: Currently there is no support for GS1/FCN1 character encoding,
//! full ECI, structured append, and
//! reader programming. The decoding output format specified in ISO/IEC 15424 is
//! also not implemented (metadata, ECI, etc.), if you have a use case for this
//! please open an issue.

#![no_std]
extern crate alloc;

mod decodation;
mod encodation;
pub mod errorcode;
pub mod placement;
mod symbol_size;

pub mod data;

pub use encodation::EncodationType;
pub use symbol_size::{SymbolList, SymbolSize};

use alloc::vec::Vec;
use flagset::FlagSet;

use encodation::DataEncodingError;
use placement::{Bitmap, MatrixMap};

#[cfg(test)]
use pretty_assertions::assert_eq;

/// Encoded Data Matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataMatrix {
    /// Size of the encoded Data Matrix
    pub size: SymbolSize,
    codewords: Vec<u8>,
    num_data_codewords: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors when decoding a Data Matrix.
pub enum DecodingError {
    PixelConversion(placement::BitmapConversionError),
    ErrorCorrection(errorcode::ErrorDecodingError),
    DataDecoding(decodation::DataDecodingError),
}

impl DataMatrix {
    /// Decode a Data Matrix from its pixels representation.
    ///
    /// The alignment pattern must be included. The argument `width` denotes the number of
    /// pixels in one row.
    ///
    /// The pixels are expected to be given in row-major order, i.e., the top
    /// row of pixels comes first, then the second row and so on.
    pub fn decode(pixels: &[bool], width: usize) -> Result<Vec<u8>, DecodingError> {
        let (matrix_map, size) =
            MatrixMap::try_from_bits(pixels, width).map_err(DecodingError::PixelConversion)?;
        let mut codewords = matrix_map.codewords();
        errorcode::decode_error(&mut codewords, size).map_err(DecodingError::ErrorCorrection)?;
        decodation::decode_data(&codewords[..size.num_data_codewords()])
            .map_err(DecodingError::DataDecoding)
    }

    /// Get the data in encoded form.
    ///
    /// Error correction is included.
    /// See [data_codewords()](Self::data_codewords) if you only need the data.
    pub fn codewords(&self) -> &[u8] {
        &self.codewords
    }

    /// Get the codewords that encode the data.
    ///
    /// This is a prefix of the codewords returned by [codewords()](Self::codewords).
    pub fn data_codewords(&self) -> &[u8] {
        &self.codewords[..self.num_data_codewords]
    }

    /// Create an abstract bitmap representing the Data Matrix.
    pub fn bitmap(&self) -> Bitmap<bool> {
        MatrixMap::new_with_codewords(&self.codewords, self.size).bitmap()
    }

    /// Encode data as a Data Matrix (ECC200).
    ///
    /// This is wrapper for [DataMatrixBuilder::encode].
    pub fn encode<I: Into<SymbolList>>(
        data: &[u8],
        symbol_list: I,
    ) -> Result<DataMatrix, DataEncodingError> {
        DataMatrixBuilder::new()
            .with_symbol_list(symbol_list)
            .encode(data)
    }

    /// Encodes a string as a Data Matrix (ECC200).
    ///
    /// This is wrapper for [DataMatrixBuilder::encode_str].
    pub fn encode_str<I: Into<SymbolList>>(
        text: &str,
        symbol_list: I,
    ) -> Result<DataMatrix, DataEncodingError> {
        DataMatrixBuilder::new()
            .with_symbol_list(symbol_list)
            .encode_str(text)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Builder for encoding a Data Matrix with more control.
pub struct DataMatrixBuilder {
    encodation_types: FlagSet<EncodationType>,
    symbol_list: SymbolList,
    use_macros: bool,
}

impl DataMatrixBuilder {
    pub fn new() -> Self {
        Self {
            encodation_types: EncodationType::all(),
            symbol_list: SymbolList::default(),
            use_macros: true,
        }
    }

    /// Specify which encodation can be used.
    ///
    /// By default all encodation types are enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use datamatrix::{DataMatrixBuilder, data::EncodationType};
    /// let datamatrix = DataMatrixBuilder::new()
    ///     .with_encodation_types(EncodationType::Base256 | EncodationType::Edifact)
    ///     .encode(b"\xFAaaa")
    ///     .unwrap();
    /// ```
    pub fn with_encodation_types(self, types: impl Into<FlagSet<EncodationType>>) -> Self {
        Self {
            encodation_types: types.into(),
            ..self
        }
    }

    /// Whether to use macros or not.
    ///
    /// This is enabled by default.
    pub fn with_macros(self, use_macros: bool) -> Self {
        Self { use_macros, ..self }
    }

    /// Specify the list of allowed symbols sizes.
    ///
    /// Uses [SymbolList::default()] by default.
    pub fn with_symbol_list<I: Into<SymbolList>>(self, symbol_list: I) -> Self {
        Self {
            symbol_list: symbol_list.into(),
            ..self
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
    /// can automatically pick the smallest size which fits the data (see [SymbolList])
    /// but there is an upper limit.
    pub fn encode(self, data: &[u8]) -> Result<DataMatrix, DataEncodingError> {
        self.encode_eci(data, None)
    }

    /// Encodes a string as a Data Matrix (ECC200).
    ///
    /// If the string can be converted to Latin-1, no ECI is used, otherwise
    /// an initial UTF8 ECI is inserted. Please check if your decoder has support
    /// for that. See the notes on the [module documentation](crate) for more details.
    pub fn encode_str(self, text: &str) -> Result<DataMatrix, DataEncodingError> {
        if let Some(data) = data::utf8_to_latin1(text) {
            // string is latin1
            self.encode_eci(&data, None)
        } else {
            // encode with UTF8 ECI
            self.encode_eci(text.as_bytes(), Some(decodation::ECI_UTF8))
        }
    }

    #[doc(hidden)]
    pub fn encode_eci(
        self,
        data: &[u8],
        eci: Option<u32>,
    ) -> Result<DataMatrix, DataEncodingError> {
        let (mut codewords, size) = data::encode_data(
            data,
            &self.symbol_list,
            eci,
            self.encodation_types,
            self.use_macros,
        )?;
        let ecc = errorcode::encode_error(&codewords, size);
        let num_data_codewords = codewords.len();
        codewords.extend_from_slice(&ecc);
        Ok(DataMatrix {
            codewords,
            size,
            num_data_codewords,
        })
    }
}

impl Default for DataMatrixBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn utf8_eci_test() {
    let data = "🥸";
    let code = DataMatrix::encode_str(data, SymbolList::default()).unwrap();
    let decoded = data::decode_str(code.data_codewords()).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_tile_placement_forth_and_back() {
    let mut rnd_data = test::random_data();
    for size in SymbolList::all() {
        let data = rnd_data(size.num_codewords());
        let map = MatrixMap::new_with_codewords(&data, size);
        assert_eq!(map.codewords(), data);
        let bitmap = map.bitmap();
        let (matrix_map, _size) = MatrixMap::try_from_bits(bitmap.bits(), bitmap.width()).unwrap();
        assert_eq!(matrix_map.codewords(), data);
    }
}

#[test]
fn test_macro_str() {
    let data = "[)>\x1E05\x1D🤘\x1E\x04";
    let map = DataMatrix::encode_str(data, SymbolList::default()).unwrap();
    let codewords = map.data_codewords();
    assert_eq!(
        codewords,
        &[
            encodation::MACRO05,
            encodation::ascii::ECI,
            decodation::ECI_UTF8 as u8 + 1,
            // Base256 encoding of the four byte utf8 character plus padding
            231,
            240,
            114,
            183,
            81,
            219,
            129,
        ]
    );
    let out = data::decode_str(codewords).unwrap();
    assert_eq!(data, out);
}

#[cfg(test)]
mod test {
    use crate::placement::MatrixMap;
    use crate::symbol_size::SymbolSize;
    use alloc::vec::Vec;

    /// Simple LCG random generator for test data generation
    pub fn random_maps() -> impl FnMut(SymbolSize) -> MatrixMap<bool> {
        let mut rnd = random_bytes();
        move |size| {
            let mut map = MatrixMap::new(size);
            map.traverse_mut(|_, bits| {
                for bit in bits {
                    *bit = rnd() > 127;
                }
            });
            map.write_padding();
            map
        }
    }

    pub fn random_bytes() -> impl FnMut() -> u8 {
        let mut seed = 0;
        move || {
            let modulus = 2u64.pow(31);
            let a = 1103515245u64;
            let c = 12345u64;
            seed = (a * seed + c) % modulus;
            (seed % 256) as u8
        }
    }

    pub fn random_data() -> impl FnMut(usize) -> Vec<u8> {
        let mut rnd = random_bytes();
        move |len| {
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                v.push(rnd());
            }
            v
        }
    }
}
