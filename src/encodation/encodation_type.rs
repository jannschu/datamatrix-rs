use flagset::{FlagSet, flags};

use super::{DataEncodingError, GenericDataEncoder, ascii, base256, c40, edifact, text, x12};

flags! {
    /// List of data encodation types
    ///
    /// Data Matrix can switch between different "codecs" in a symbol. Each one
    /// has its strengths and weaknesses.
    pub enum EncodationType: u8 {
        Ascii   = 0b000001,
        C40     = 0b000010,
        Text    = 0b000100,
        X12     = 0b001000,
        Edifact = 0b010000,
        Base256 = 0b100000,
    }
}

impl EncodationType {
    /// Get a fixed index between 0 and 5 for the encodation type.
    ///
    /// The index also encodes the preferences of encodation types,
    /// where lower numbers are better.
    pub fn index(&self) -> usize {
        match self {
            // Order is chosen based on my personal
            // estimate of which modes are more complicated.
            Self::Ascii => 0,
            Self::Base256 => 1,
            Self::Edifact => 2,
            Self::X12 => 3,
            Self::C40 => 4,
            Self::Text => 5,
        }
    }

    /// Get flag set with all encodation types activated.
    pub fn all() -> FlagSet<Self> {
        FlagSet::full()
    }

    pub(super) fn encode<'a, 'b: 'a>(
        &self,
        encoder: &'a mut GenericDataEncoder<'b>,
    ) -> Result<(), DataEncodingError> {
        match self {
            Self::Ascii => ascii::encode(encoder),
            Self::C40 => c40::encode(encoder),
            Self::Text => text::encode(encoder),
            Self::X12 => x12::encode(encoder),
            Self::Edifact => edifact::encode(encoder),
            Self::Base256 => base256::encode(encoder),
        }
    }

    pub(super) fn is_ascii(&self) -> bool {
        matches!(self, EncodationType::Ascii)
    }

    /// Get the LATCH codeword to switch to this mode from ASCII.
    pub(super) fn latch_from_ascii(&self) -> u8 {
        match self {
            Self::Ascii => panic!("can not switch from ascii to ascii"),
            Self::C40 => ascii::LATCH_C40,
            Self::Text => ascii::LATCH_TEXT,
            Self::X12 => ascii::LATCH_X12,
            Self::Edifact => ascii::LATCH_EDIFACT,
            Self::Base256 => ascii::LATCH_BASE256,
        }
    }
}
