use std::iter::Cloned;
use std::slice::Iter;

use super::{encodation_type::EncodationType, Encoder, EncodingContext, GenericEncoder};
use crate::symbol_size::{Capacity, Size, SymbolSize};

pub(super) trait TestEncoderLogic: Sized {
    type State;

    fn maybe_switch_mode(enc: &mut TestEncodingContext<Self>) -> bool;

    fn symbol_size_left(
        enc: &mut TestEncodingContext<Self>,
        extra_codewords: usize,
    ) -> Option<usize>;
}

pub(super) struct DummyLogic(Option<usize>, isize);

impl DummyLogic {
    pub fn new(
        data: Vec<u8>,
        // The size of the symbol
        size: usize,
        // countdown in maybe_switch_mode, decremented each call,
        // if zero return true
        count: isize,
    ) -> TestEncodingContext<Self> {
        TestEncodingContext::new(data, (size, count))
    }
}

impl TestEncoderLogic for DummyLogic {
    type State = (usize, isize);

    fn maybe_switch_mode(enc: &mut TestEncodingContext<Self>) -> bool {
        enc.state.1 -= 1;
        if enc.state.1 == 0 {
            true
        } else {
            false
        }
    }

    fn symbol_size_left(
        enc: &mut TestEncodingContext<Self>,
        extra_codewords: usize,
    ) -> Option<usize> {
        let needed = enc.codewords().len() + extra_codewords;
        if needed > enc.state.0 {
            None
        } else {
            Some(enc.state.0 - needed)
        }
    }
}

pub(super) struct TestEncodingContext<T: TestEncoderLogic> {
    pub(super) removed: Vec<u8>,
    pub(super) data: Vec<u8>,
    pub(super) codewords: Vec<u8>,
    pub(super) mode: EncodationType,
    state: T::State,
}

impl<T: TestEncoderLogic> TestEncodingContext<T> {
    pub fn new(data: Vec<u8>, state: T::State) -> Self {
        Self {
            data,
            codewords: Vec::new(),
            state,
            mode: EncodationType::Ascii,
            removed: Vec::new(),
        }
    }
}

impl<T: TestEncoderLogic> EncodingContext for TestEncodingContext<T> {
    fn maybe_switch_mode(&mut self) -> bool {
        T::maybe_switch_mode(self)
    }

    fn symbol_size_left(&mut self, extra_codewords: usize) -> Option<usize> {
        T::symbol_size_left(self, extra_codewords)
    }

    fn eat(&mut self) -> Option<u8> {
        if self.data.is_empty() {
            None
        } else {
            let removed = self.data.remove(0);
            self.removed.push(removed);
            Some(removed)
        }
    }

    fn backup(&mut self, steps: usize) {
        for i in self.removed.iter().rev().take(steps) {
            self.data.insert(0, *i);
        }
    }

    fn rest(&self) -> &[u8] {
        &self.data
    }

    fn push(&mut self, ch: u8) {
        self.codewords.push(ch);
    }

    fn replace(&mut self, index: usize, ch: u8) {
        self.codewords[index] = ch;
    }

    fn insert(&mut self, index: usize, ch: u8) {
        self.codewords.insert(index, ch);
    }

    fn set_mode(&mut self, mode: super::encodation_type::EncodationType) {
        self.mode = mode;
    }

    fn codewords(&self) -> &[u8] {
        &self.codewords
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TestSymbol {
    // error codewords, matrix width, matrix height, data regions
    Square3,  // 5, 8, 8, 1),
    Square5,  // 7, 10, 10, 1),
    Rect5,    // 7, 16, 6, 1),
    Square8,  // 10, 12, 12, 1),
    Rect10,   // 11, 14, 6, 2),
    Square13, // 0, 0, 0, 1),
    Square77, // 0, 0, 0, 1)
    Auto,
}

#[rustfmt::skip]
static SYMBOLS: [TestSymbol; 7] = [
    TestSymbol::Square3, TestSymbol::Square5, TestSymbol::Rect5, TestSymbol::Square8,
    TestSymbol::Rect10, TestSymbol::Square13, TestSymbol::Square77,
];

impl Size for TestSymbol {
    const DEFAULT: Self = TestSymbol::Auto;

    fn num_data_codewords(&self) -> Option<usize> {
        match self {
            Self::Square3 => Some(3),
            Self::Square5 => Some(5),
            Self::Rect5 => Some(5),
            Self::Square8 => Some(8),
            Self::Rect10 => Some(10),
            Self::Square13 => Some(13),
            Self::Square77 => Some(77),
            Self::Auto => None,
        }
    }

    fn candidates(&self) -> Cloned<Iter<Self>> {
        if self == &Self::Auto {
            return SYMBOLS.iter().cloned();
        }
        let index = SYMBOLS
            .iter()
            .enumerate()
            .find(|(_i, size)| size == &self)
            .unwrap()
            .0;
        SYMBOLS[index..index + 1].iter().cloned()
    }

    fn max_codeswords(&self) -> usize {
        if let Some(num) = self.num_data_codewords() {
            return num;
        }
        77
    }

    fn max_capacity(&self) -> Capacity {
        match self {
            Self::Square3 => Capacity::new(6, 2),
            Self::Square5 => Capacity::new(10, 4),
            Self::Rect5 => Capacity::new(10, 4),
            Self::Square8 => Capacity::new(16, 7),
            Self::Rect10 => Capacity::new(20, 9),
            Self::Square13 => Capacity::new(26, 12),
            Self::Square77 | Self::Auto => Capacity::new(154, 76),
        }
    }
}

type TestEncoder<'a> = GenericEncoder<'a, TestSymbol>;

#[cfg(test)]
fn enc(data: &[u8]) -> Vec<u8> {
    Encoder::with_size(data, SymbolSize::Auto)
        .codewords()
        .unwrap()
}

#[test]
#[ignore]
fn test_edifact_xzing_issue() {
    // See https://github.com/zxing/zxing/issues/624
    // See https://github.com/zxing/zxing/issues/1335 ("<03>TILSIT-MUNSTER<05>Paula")
    // See https://github.com/zxing/zxing/issues/986
    // See https://github.com/zxing/zxing/issues/960
    // See https://github.com/zxing/zxing/issues/912
    // See https://github.com/zxing/zxing/issues/908
    todo!()
}

#[test]
fn test_ascii_encodation_two_digits() {
    assert_eq!(enc(b"123456"), vec![142, 164, 186]);
}

#[test]
fn test_ascii_encodation_two_digits_with_upper() {
    assert_eq!(enc(b"123456\xa3"), vec![142, 164, 186, 235, 36]);
}

#[test]
fn test_ascii_encodation_example1() {
    assert_eq!(
        enc(b"30Q324343430794<OQQ"),
        vec![160, 82, 162, 173, 173, 173, 137, 224, 61, 80, 82, 82]
    );
}

#[test]
fn test_c40_basic1() {
    assert_eq!(enc(b"AIMAIMAIM"), vec![230, 91, 11, 91, 11, 91, 11, 254]);
}

#[test]
fn test_c40_basic2_1() {
    // "B" is normally encoded as "15" (one C40 value)
    // "else" case: "B" is encoded as ASCII
    assert_eq!(enc(b"AIMAIAB"), vec![230, 91, 11, 90, 255, 254, 67, 129]);
}

#[test]
fn test_c40_basic2_2() {
    // Encoded as ASCII
    // Alternative solution:
    // assert_eq!(words, vec![230, 91, 11, 90, 255, 254, 99, 129]);
    // "b" is normally encoded as "Shift 3, 2" (two C40 values)
    // "else" case: "b" is encoded as ASCII
    assert_eq!(enc(b"AIMAIAb"), vec![66, 74, 78, 66, 74, 66, 99, 129]);
}

#[test]
fn test_c40_basic2_3() {
    assert_eq!(
        enc(b"AIMAIMAIM\xcb"),
        vec![230, 91, 11, 91, 11, 91, 11, 254, 235, 76]
    );
    // Alternative solution:
    // assert_eq!(words, vec![230, 91, 11, 91, 11, 91, 11, 11, 9, 254]);
    // Expl: 230 = shift to C40, "91, 11" = "AIM",
    // "11, 9" = "\xcb" = "Shift 2, UpperShift, <char>
    // "else" case
}

#[test]
fn test_c40_basic2_4() {
    assert_eq!(
        enc(b"AIMAIMAIM\xeb"),
        vec![230, 91, 11, 91, 11, 91, 11, 254, 235, 108]
    );
    // Activate when additional rectangulars are available
    // Expl: 230 = shift to C40, "91, 11" = "AIM",
    // "\xeb" in C40 encodes to: 1, 30, 2, 11 which doesn't fit into a triplet
    // "10, 243" =
    // 254 = unlatch, 235 = Upper Shift, 108 = 0xEB/235 - 128 + 1
    // "else" case
}

#[test]
fn test_c40_spec_example() {
    assert_eq!(
        enc(b"A1B2C3D4E5F6G7H8I9J0K1L2"),
        vec![230, 88, 88, 40, 8, 107, 147, 59, 67, 126, 206, 78, 126, 144, 121, 35, 47, 254]
    );
}

#[test]
fn test_c40_special_case_a() {
    // case "a": Unlatch is not required
    let words = TestEncoder::new(b"AIMAIMAIMAIMAIMAIM").codewords().unwrap();
    assert_eq!(
        words,
        vec![230, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11]
    );
}

#[test]
fn test_c40_special_case_b() {
    // case "b": Add trailing shift 0 and Unlatch is not required
    let words = TestEncoder::new(b"AIMAIMAIMAIMAIMAI").codewords().unwrap();
    assert_eq!(
        words,
        vec![230, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 90, 241]
    );
}

#[test]
fn test_c40_special_case_c() {
    //case "c": Unlatch and write last character in ASCII
    let words = TestEncoder::new(b"AIMAIMAIMAIMAIMA").codewords().unwrap();
    assert_eq!(
        words,
        vec![230, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 254, 66]
    );
}

#[test]
fn test_c40_partial_triple() {
    // Encode A I M using paris of C40 values, until only 'A' and 'I' is left.
    // In this case, UNLATCH and encode AI as ASCII to avoid partial triple.
    assert_eq!(
        enc(b"AIMAIMAIMAIMAIMAI"),
        vec![230, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 254, 66, 74, 129, 237]
    );
}

#[test]
fn test_c40_special_case_d() {
    // case "d": Skip Unlatch and write last character in ASCII
    assert_eq!(enc(b"AIMAIMAIMA"), vec![230, 91, 11, 91, 11, 91, 11, 66]);
}

#[test]
fn test_c40_special_cases2() {
    // available > 2, rest = 2 --> unlatch and encode as ASCII
    assert_eq!(
        enc(b"AIMAIMAIMAIMAIMAIMAI"),
        vec![230, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 254, 66, 74]
    );
}

#[test]
fn test_text_encoding_1() {
    // 239 shifts to Text encodation, 254 unlatches
    let words = Encoder::with_size(b"aimaimaim", SymbolSize::Auto)
        .codewords()
        .unwrap();
    assert_eq!(words, vec![239, 91, 11, 91, 11, 91, 11, 254]);
}

#[test]
fn test_text_encoding_2() {
    assert_eq!(
        enc(b"aimaimaim'"),
        vec![239, 91, 11, 91, 11, 91, 11, 254, 40, 129]
    );
    // This is an alternative, but doesn't strictly follow the rules in the spec.
    // assertEquals("239, 91, 11, 91, 11, 91, 11, 7, 49, 254", visualized);
}

#[test]
fn test_text_encoding_3() {
    assert_eq!(enc(b"aimaimaIm"), vec![239, 91, 11, 91, 11, 87, 218, 110]);
}

#[test]
fn test_text_encoding_4() {
    assert_eq!(
        enc(b"aimaimaimB"),
        vec![239, 91, 11, 91, 11, 91, 11, 254, 67, 129]
    );
}

#[test]
fn test_text_encoding_5() {
    assert_eq!(
        enc(b"aimaimaim{txt}\x04"),
        vec![239, 91, 11, 91, 11, 91, 11, 16, 218, 236, 107, 181, 69, 254, 129, 237]
    );
}

#[test]
fn test_x12_1() {
    // 238 shifts to X12 encodation, 254 unlatches
    assert_eq!(
        enc(b"ABC>ABC123>AB"),
        vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 67,]
    );
}

#[test]
fn test_x12_2() {
    assert_eq!(
        enc(b"ABC>ABC123>ABC"),
        vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 254, 67, 68]
    );
}

#[test]
fn test_x12_3() {
    assert_eq!(
        enc(b"ABC>ABC123>ABCD"),
        vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 96, 82, 254]
    );
}

#[test]
fn test_x12_4() {
    assert_eq!(
        enc(b"ABC>ABC123>ABCDE"),
        vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 96, 82, 70]
    );
}

#[test]
fn test_x12_5() {
    assert_eq!(
        enc(b"ABC>ABC123>ABCDEF"),
        vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 96, 82, 254, 70, 71, 129, 237]
    );
}

#[test]
fn test_edifact_1() {
    // 240 shifts to EDIFACT encodation
    assert_eq!(
        enc(b".A.C1.3.DATA.123DATA.123DATA"),
        vec![
            240, 184, 27, 131, 198, 236, 238, 16, 21, 1, 187, 28, 179, 16, 21, 1, 187, 28, 179, 16,
            21, 1
        ]
    );
}

#[test]
fn test_edifact_2() {
    assert_eq!(
        enc(b".A.C1.3.X.X2.."),
        vec![240, 184, 27, 131, 198, 236, 238, 98, 230, 50, 47, 47]
    );
}

#[test]
fn test_edifact_3() {
    assert_eq!(
        enc(b".A.C1.3.X.X2."),
        vec![240, 184, 27, 131, 198, 236, 238, 98, 230, 50, 47, 129]
    );
}

#[test]
fn test_edifact_4() {
    assert_eq!(
        enc(b".A.C1.3.X.X2"),
        vec![240, 184, 27, 131, 198, 236, 238, 98, 230, 50]
    );
}

#[test]
fn test_edifact_5() {
    assert_eq!(
        enc(b".A.C1.3.X.X"),
        vec![240, 184, 27, 131, 198, 236, 238, 98, 230, 31]
    );
}

#[test]
fn test_edifact_6() {
    assert_eq!(
        enc(b".A.C1.3.X."),
        vec![240, 184, 27, 131, 198, 236, 238, 98, 231, 192]
    );
}

#[test]
fn test_edifact_7() {
    assert_eq!(
        enc(b".A.C1.3.X"),
        vec![240, 184, 27, 131, 198, 236, 238, 89]
    );
}

#[test]
fn test_edifact_8() {
    //Checking temporary unlatch from EDIFACT
    assert_eq!(
        enc(b".XXX.XXX.XXX.XXX.XXX.XXX.\xFCXX.XXX.XXX.XXX.XXX.XXX.XXX"),
        vec![
            240, 185, 134, 24, 185, 134, 24, 185, 134, 24, 185, 134, 24, 185, 134, 24, 185, 134,
            24,
            // 124 == UNLATCH << 2 (so edifact encoding of single value UNLATCH)
            124, 47, 235, 125, 240, 97, 139, 152, 97, 139, 152, 97, 139, 152, 97, 139, 152, 97, 139,
            152, 97, 139, 152, 89, 89
        ]
    );
}

#[cfg(test)]
fn create_binary_test_message(len: usize) -> Vec<u8> {
    let mut vec = vec![171, 228, 246, 252, 233, 224, 225, 45];
    for _ in 0..len - 9 {
        vec.push(b'\xB7');
    }
    vec.push(b'\xBB');
    vec
}

#[test]
fn test_base256_1() {
    // 231 shifts to Base256 encodation
    assert_eq!(
        enc(b"\xab\xe4\xf6\xfc\xe9\xbb"),
        vec![231, 44, 108, 59, 226, 126, 1, 104]
    );
}

#[test]
fn test_base256_2() {
    assert_eq!(
        enc(b"\xab\xe4\xf6\xfc\xe9\xe0\xbb"),
        vec![231, 51, 108, 59, 226, 126, 1, 141, 254, 129]
    );
}

#[test]
fn test_base256_3() {
    assert_eq!(
        enc(b"\xab\xe4\xf6\xfc\xe9\xe0\xe1\xbb"),
        vec![231, 44, 108, 59, 226, 126, 1, 141, 36, 147]
    );
}

#[test]
fn test_base256_4() {
    // ASCII only (for reference)
    assert_eq!(enc(b" 23\xa3"), vec![33, 153, 235, 36, 129]);
}

#[test]
fn test_base256_5() {
    // Mixed Base256 + ASCII
    assert_eq!(
        enc(b"\xab\xe4\xf6\xfc\xe9\xbb 234"),
        vec![231, 51, 108, 59, 226, 126, 1, 104, 99, 153, 53, 129]
    );
}

#[test]
fn test_base256_6() {
    assert_eq!(
        enc(b"\xab\xe4\xf6\xfc\xe9\xbb 23\xa3 1234567890123456789"),
        vec![
            231, 55, 108, 59, 226, 126, 1, 104, 99, 10, 161, 167, 185, 142, 164, 186, 208, 220,
            142, 164, 186, 208, 58, 129, 59, 209, 104, 254, 150, 45
        ]
    );
}

#[test]
fn test_base256_7() {
    //padding necessary at the end
    assert_eq!(
        enc(&create_binary_test_message(20)),
        vec![
            231, 44, 108, 59, 226, 126, 1, 141, 36, 5, 37, 187, 80, 230, 123, 17, 166, 60, 210,
            103, 253, 150
        ]
    );
}

#[test]
fn test_base256_8() {
    assert_eq!(
        enc(&create_binary_test_message(19)),
        vec![
            231, 63, 108, 59, 226, 126, 1, 141, 36, 5, 37, 187, 80, 230, 123, 17, 166, 60, 210,
            103, 1, 129
        ],
    );
}

#[test]
fn test_base256_9() {
    let words = enc(&create_binary_test_message(276));
    let start = vec![231, 38, 219, 2, 208, 120, 20, 150, 35];
    assert_eq!(&words[..start.len()], &start);
    let end = vec![146, 40, 194, 129];
    assert_eq!(&words[words.len() - end.len()..], &end);
}

#[test]
fn test_base256_10() {
    let words = enc(&create_binary_test_message(277));
    let start = vec![231, 38, 220, 2, 208, 120, 20, 150, 35];
    assert_eq!(&words[..start.len()], &start);
    let end = vec![146, 40, 190, 87];
    assert_eq!(&words[words.len() - end.len()..], &end);
}

#[test]
fn test_unlatching_from_c40() {
    assert_eq!(
        enc(b"AIMAIMAIMAIMaimaimaim"),
        vec![230, 91, 11, 91, 11, 91, 11, 254, 66, 74, 78, 239, 91, 11, 91, 11, 91, 11]
    );
}

#[test]
fn test_unlatching_from_text() {
    assert_eq!(
        enc(b"aimaimaimaim12345678"),
        vec![239, 91, 11, 91, 11, 91, 11, 91, 11, 254, 142, 164, 186, 208, 129, 237]
    );
}

#[test]
fn test_hello_world() {
    assert_eq!(
        enc(b"Hello World!"),
        // zxing has 233 instead of 234 because of the different
        // backstep behavior in c40, we cut values they zero them
        vec![73, 239, 116, 130, 175, 123, 148, 64, 158, 234, 254, 34]
    );
}

#[test]
fn test_bug_1664266() {
    // There was an exception and the encoder did not handle the unlatching from
    // EDIFACT encoding correctly
    assert_eq!(
        enc(b"CREX-TAN:h"),
        vec![240, 13, 33, 88, 181, 64, 78, 124, 59, 105]
    );
    assert_eq!(
        enc(b"CREX-TAN:hh"),
        vec![240, 13, 33, 88, 181, 64, 78, 124, 59, 105, 105, 129]
    );
    assert_eq!(
        enc(b"CREX-TAN:hhh"),
        vec![240, 13, 33, 88, 181, 64, 78, 124, 59, 105, 105, 105]
    );
}

#[test]
fn test_x12_unlatch() {
    assert_eq!(enc(b"*DTCP01"), vec![238, 9, 10, 104, 141, 254, 50, 129]);
}

#[test]
fn test_x12_unlatch_2() {
    assert_eq!(enc(b"*DTCP0"), vec![238, 9, 10, 104, 141]);
}

#[test]
fn test_bug_3048549() {
    // There was an IllegalArgumentException for an illegal character here because
    // of an encoding problem of the character 0x0060 in Java source code.
    assert_eq!(
        enc(b"fiykmj*Rh2`,e6"),
        vec![239, 122, 87, 154, 40, 7, 171, 115, 207, 12, 130, 71, 155, 254, 129, 237]
    );
}
