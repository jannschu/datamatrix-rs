use super::{ascii, base256, c40, edifact, text, x12, EncodationError, GenericEncoder};
use crate::symbol_size::Size;

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) enum EncodationType {
    Ascii,
    C40,
    Text,
    X12,
    Edifact,
    Base256,
}

impl EncodationType {
    pub(super) fn encode<'a, 'b: 'a, S: Size>(
        &self,
        encoder: &'a mut GenericEncoder<'b, S>,
    ) -> Result<(), EncodationError> {
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
