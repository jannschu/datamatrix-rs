use super::{ascii, base256, c40, edifact, text, x12, DataEncodingError, GenericDataEncoder};

#[derive(Debug, Copy, Clone, PartialEq)]
/// List of data encodation types
///
/// Data Matrix can switch between different "codecs" in a symbol. Each one
/// has its strenghts and weaknesses.
pub enum EncodationType {
    Ascii,
    C40,
    Text,
    X12,
    Edifact,
    Base256,
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
