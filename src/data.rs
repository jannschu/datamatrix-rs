//! Data part de- and encoding (exposure of some internal functionality).
//!
//! The bytes encoded into a DataMatrix consist of the actual data,
//! and error correction bytes.
//!
//! The functions in this module can be used to run
//! the data part of the full decoding or encoding process.
pub use crate::decodation::{decode_data, DataDecodingError};
pub use crate::encodation::{DataEncoder, DataEncodingError};
