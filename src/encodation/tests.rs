use alloc::{vec, vec::Vec};

use flagset::FlagSet;

use super::{encodation_type::EncodationType, DataEncodingError, EncodingContext};
use crate::data::encode_data;
use crate::symbol_size::SymbolList;

#[cfg(test)]
use pretty_assertions::assert_eq;

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
        enc.state.1 == 0
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
    fn maybe_switch_mode(&mut self) -> Result<bool, DataEncodingError> {
        Ok(T::maybe_switch_mode(self))
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

    fn set_ascii_until_end(&mut self) {
        self.mode = EncodationType::Ascii;
    }

    fn codewords(&self) -> &[u8] {
        &self.codewords
    }
}

#[cfg(test)]
fn enc(data: &[u8]) -> Vec<u8> {
    enc_mode(data, EncodationType::all())
}

#[cfg(test)]
fn enc_mode(data: &[u8], enabled_modes: impl Into<FlagSet<EncodationType>>) -> Vec<u8> {
    encode_data(
        data,
        &SymbolList::default(),
        None,
        enabled_modes.into(),
        false,
    )
    .unwrap()
    .0
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
fn test_c40_basic2_3() {
    assert_eq!(
        enc(b"AIMAIMAIM\xcb"),
        vec![230, 91, 11, 91, 11, 91, 11, 11, 9, 254],
    );
    // Alternative solution:
    // assert_eq!(words, vec![230, 91, 11, 91, 11, 91, 11, 254, 235, 76]);
    // Expl: 230 = shift to C40, "91, 11" = "AIM",
    // "11, 9" = "\xcb" = "Shift 2, UpperShift, <char>
    // "else" case
}

#[test]
fn test_c40_spec_example() {
    assert_eq!(
        enc(b"A_2_D_5_G7H_9J_1L2"),
        vec![230, 87, 195, 37, 195, 106, 131, 56, 131, 126, 206, 10, 94, 144, 3, 35, 47, 254],
        // Alternatives:
        // vec![66, 96, 51, 96, 69, 96, 230, 56, 131, 126, 206, 10, 94, 144, 3, 35, 47, 254],
        // vec![66, 96, 51, 96, 69, 96, 54, 96, 230, 126, 206, 10, 94, 144, 3, 35, 47, 254],
        // vec![230, 88, 88, 40, 8, 107, 147, 59, 67, 126, 206, 78, 126, 144, 121, 35, 47, 254]
    );
}

#[test]
fn test_c40_special_case_a() {
    // case "a": Unlatch is not required
    assert_eq!(enc(b"lvzvlv"), vec![239, 161, 224, 222, 204]);
}

#[test]
fn test_c40_special_case_b() {
    // case "b": Add trailing shift 0 and Unlatch is not required
    assert_eq!(
        enc(b"\x83)nnnnnnnn\xb8"),
        vec![235, 4, 42, 239, 173, 20, 173, 20, 172, 250, 189, 97]
    );
}

#[test]
fn test_c40_special_case_c() {
    //case "c": Unlatch and write last character in ASCII
    assert_eq!(
        enc(b"?      T        \xda  \x10"),
        vec![64, 230, 19, 60, 19, 60, 206, 188, 19, 60, 19, 60, 11, 24, 19, 57, 254, 17],
    );
}

#[test]
fn test_c40_special_case_d() {
    // case "d": Skip Unlatch and write last character in ASCII
    assert_eq!(enc(b"    \x1d    "), vec![230, 19, 60, 18, 222, 19, 60, 33]);
}

#[test]
fn test_c40_special_case2_d() {
    // case "d": Skip Unlatch and write last two digits in ASCII
    assert_eq!(
        enc(b" 9 aaabbb00"),
        vec![239, 20, 204, 89, 191, 96, 40, 130]
    );
}

#[test]
fn test_c40_special_cases2() {
    // available > 2, rest = 2 --> unlatch and encode as ASCII
    assert_eq!(
        enc(b"aimaimaimaimaimaimai"),
        vec![239, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 91, 11, 90, 242, 254]
    );
}

#[test]
fn test_text_encoding_1() {
    // 239 shifts to Text encodation, 254 unlatches
    let words = encode_data(
        b"aimaimaim",
        &SymbolList::default(),
        None,
        EncodationType::all(),
        false,
    )
    .unwrap()
    .0;
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
        enc(b"AB\x0d>ABC123>AB"),
        vec![238, 89, 217, 14, 192, 100, 207, 44, 31, 67,]
    );
}

#[test]
fn test_x12_2a() {
    assert_eq!(
        enc(b"AB\x0d>ABC123>ABC"),
        // BC will remain as an incomplete triple in X12,
        // end rule does not apply
        vec![238, 89, 217, 14, 192, 100, 207, 44, 31, 254, 67, 68]
    );
}

#[test]
fn test_x12_2b() {
    assert_eq!(
        enc(b"AB\x0d>ABC123>A00"),
        // 00 will remain as an incomplete triple in X12,
        // end rule does apply, can be encoded as one ASCII (130)
        vec![238, 89, 217, 14, 192, 100, 207, 44, 31, 130]
    );
}

#[test]
fn test_x12_3() {
    assert_eq!(
        enc(b"AB\x0d>ABC123>ABCD"),
        vec![238, 89, 217, 14, 192, 100, 207, 44, 31, 96, 82, 254]
    );
}

#[test]
fn test_x12_4() {
    assert_eq!(
        enc(b"ABC>ABC123>ABCDE"),
        vec![
            238, // UNLATCH
            89, 233, // ABC
            14, 192, // >AB
            100, 207, // C12
            44, 31, // 3>A
            96, 82, // BCD
            70  // E (ASCII)
        ]
    );
}

#[test]
fn test_x12_5() {
    assert_eq!(
        enc(b"ABC>ABC123>ABCDEF"),
        vec![240, 4, 32, 254, 4, 32, 241, 203, 63, 129, 8, 49, 5, 25, 240, 129],
        // Alternative:
        // vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 96, 82, 254, 70, 71, 129, 237],
        // vec![238, 89, 233, 14, 192, 100, 207, 44, 31, 254, 230, 96, 82, 254, 70, 71]
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
        // 240 LATCH
        // ".A.C" 184 27 131
        // "1.3." 198 236 238
        // X." 98 231 192
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
            24, 185, 240, 235, 125, 240, 97, 139, 152, 97, 139, 152, 97, 139, 152, 97, 139, 152,
            97, 139, 152, 97, 139, 152, 89, 89
        ]
    );
}

#[cfg(test)]
fn create_binary_test_message(len: usize) -> Vec<u8> {
    let mut vec = vec![171, 228, 246, 252, 233, 224, 225, 45];
    vec.resize(len - 1, b'\xB7');
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
        vec![231, 51, 108, 59, 226, 126, 1, 104, 99, 153, 53, 129],
        // Alternative:
        // vec![231, 50, 108, 59, 226, 126, 1, 104, 33, 153, 53, 129]
    );
}

#[test]
fn test_base256_6() {
    assert_eq!(
        enc(b"\xab\xe4\xf6\xfc\xe9\xbb 23\xa3 1234567890123456789"),
        vec![
            231, 55, 108, 59, 226, 126, 1, 104, 99, 10, 161, 167, 185, 142, 164, 186, 208, 220,
            142, 164, 186, 208, 58, 129, 59, 209, 104, 254, 150, 45
        ],
        // Alternative:
        // vec![
        //     231, 51, 108, 59, 226, 126, 1, 104, 99, 153, 235, 36, 33, 142, 164,
        //     186, 208, 220, 142, 164, 186, 208, 58, 129, 59, 209, 104, 254, 150, 45,
        // ]
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
fn test_c40_unlatching() {
    assert_eq!(
        enc(b"AIMAIMAIMAIMaimaimaim"),
        vec![230, 91, 11, 91, 11, 91, 11, 91, 11, 254, 239, 91, 11, 91, 11, 91, 11, 254]
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
    let out = enc(b"Hello World!");
    assert_eq!(
        out,
        vec![73, 239, 116, 130, 175, 123, 148, 64, 158, 234, 254, 34]
    );
}

#[test]
fn test_edifact_short() {
    assert_eq!(enc(b"CR%X-----"), vec![240, 13, 41, 88, 182, 219, 109, 46]);
}

#[test]
fn test_ascii_short() {
    assert_eq!(
        // no need to use EDIFACT, ASCII also has 5
        enc(b"CR%X-"),
        vec![68, 83, 38, 89, 46]
    );
}

#[test]
fn test_bug_1664266_1() {
    assert_eq!(
        enc(b"CREX-TAN:h"),
        vec![68, 83, 70, 89, 46, 85, 66, 79, 59, 105],
        // Alternative: C40
        // vec![230, 104, 235, 231, 117, 208, 140, 8, 155, 105],
        // Alternative: EDIFACT
        // vec![240, 13, 33, 88, 181, 64, 78, 124, 59, 105]
    );
}

#[test]
fn test_bug_1664266_2() {
    assert_eq!(
        enc(b"CREX-TAN:hh"),
        vec![68, 83, 70, 89, 46, 85, 66, 79, 59, 105, 105, 129],
        // Alternative: EDIFACT
        // vec![230, 104, 235, 231, 117, 208, 140, 8, 155, 50, 89, 254],
        // vec![240, 13, 33, 88, 181, 64, 78, 124, 59, 105, 105, 129]
    );
}

#[test]
fn test_bug_1664266_3() {
    assert_eq!(
        enc(b"CREX-TAN:hhh"),
        vec![68, 83, 70, 89, 46, 85, 66, 79, 59, 105, 105, 105],
        // Alternative
        // vec![68, 83, 70, 89, 46, 85, 66, 79, 59, 239, 134, 158],
        // vec![240, 13, 33, 88, 181, 64, 78, 124, 59, 105, 105, 105]
    );
}

#[test]
fn test_x12_unlatch_ascii() {
    assert_eq!(
        enc(b"*\x0d*******00"),
        vec![238, 6, 66, 6, 106, 6, 106, 130]
    );
}

#[test]
fn test_x12_unlatch_2() {
    assert_eq!(enc(b"*\x0dTCP0"), vec![238, 6, 98, 104, 141]);
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

#[test]
fn test_only_base256() {
    assert_eq!(
        enc_mode(b"01", EncodationType::Base256),
        vec![231, 46, 241, 136, 129],
    );
}

#[test]
fn test_only_edifact() {
    assert_eq!(
        enc_mode(b"01", EncodationType::Edifact),
        vec![240, 131, 129],
    );
}

#[test]
fn test_only_edifact_impossible() {
    let code = encode_data(
        b"aaa",
        &SymbolList::default(),
        None,
        EncodationType::Edifact,
        false,
    );
    assert_eq!(code, Err(DataEncodingError::TooMuchOrIllegalData),);
}
