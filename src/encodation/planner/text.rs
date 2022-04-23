use super::c40::{C40LikePlan, CharsetInfo};
use crate::encodation::text;

#[cfg(test)]
use pretty_assertions::assert_eq;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TextCharset;

impl CharsetInfo for TextCharset {
    fn val_size(ch: u8) -> u8 {
        text::val_size(ch)
    }

    fn in_base_set(ch: &u8) -> bool {
        text::in_base_set(*ch)
    }
}

pub(super) type TextPlan<T> = C40LikePlan<T, TextCharset>;

#[test]
fn text_val_size_205() {
    assert_eq!(TextCharset::val_size(205), 4);
}
