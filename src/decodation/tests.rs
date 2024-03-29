/// Module contains test cases found by fuzzing using libfuzzer and afl.
///
/// I now believe in magic. Holy. Shit. 10/10.
use crate::data::{decode_data, encode_data};
use crate::{EncodationType, SymbolList, SymbolSize};

use alloc::vec;

#[cfg(test)]
use pretty_assertions::assert_eq;

fn forth_and_back(data: &[u8]) -> Option<SymbolSize> {
    let encoded = encode_data(
        data,
        &SymbolList::default(),
        None,
        EncodationType::all(),
        false,
    );
    if let Ok(encoded) = encoded {
        let decoded = decode_data(&encoded.0);
        assert!(
            decoded.is_ok(),
            "err: {:?}, encoded: {:?}",
            &decoded,
            &encoded
        );
        assert_eq!(data, &decoded.unwrap(), "encoded: {:?}", &encoded);
        // println!("encoded: {:?}", &encoded);
        Some(encoded.1)
    } else {
        assert!(
            data.len() > 1_555,
            "should have fit, error is: {:?}",
            encoded
        );
        None
    }
}

#[test]
fn regression_zxing() {
    // We collect some inputs which were reported to cause
    // problems with zxing.
    //
    // Our planner works different to zxing's, we do not
    // pick the same encodation types, so we might not run
    // into those reported issues. In some cases that is part of the
    // solution though.

    // See https://github.com/zxing/zxing/issues/624
    forth_and_back(b"test TE>240 2 I.E ST>300");
    // See https://github.com/zxing/zxing/issues/1335
    forth_and_back(b"<03>TILSIT-MUNSTER<05>Paula");
    // See https://github.com/zxing/zxing/issues/986
    forth_and_back(b"**10074938*Q6000*P85005-FLT003*RA*0*K110775*VKAR99AL*1T100749381**");
    // See https://github.com/zxing/zxing/issues/960 and
    // https://github.com/zxing/zxing/issues/912 and
    // https://github.com/zxing/zxing/issues/908
    forth_and_back(b"https://test~[******]_");
    forth_and_back(b"abc<->ABCDE");
    forth_and_back(b"<ABCDEFG><ABCDEFGK>");
    forth_and_back(b"*CH/GN1/022/00");

    // See https://github.com/woo-j/OkapiBarcode/issues/80
    forth_and_back(b"02900002608229JDZ*9P0AD8AWFRB");
    // See https://github.com/woo-j/OkapiBarcode/issues/21
    assert_eq!(forth_and_back(b"9HR3Z6"), Some(SymbolSize::Square12));
}

#[test]
fn regression_iec16022() {
    // https://github.com/rdoeffinger/iec16022/issues/2
    forth_and_back(b"UEXPLR4-CBR3A3-001-TSK 13471 3216");
    // https://github.com/rdoeffinger/iec16022/issues/15
    forth_and_back(b"10000000000&AA0000&000000000000&#FFFFFFFFFFFF&00:00:00:A3:C5:62");
}

#[test]
fn regression1() {
    // Generates two big C40
    forth_and_back(&[50, 32, 32, 252]);
    assert_eq!(
        decode_data(&[51, 239, 19, 58, 187, 237, 254, 254]),
        Ok(vec![50, 32, 32, 252])
    );
}

#[test]
fn regression2() {
    forth_and_back(&[10, 66, 56, 138]);
    assert_eq!(
        decode_data(&[11, 230, 95, 162, 187, 139, 254, 254]),
        Ok(vec![10, 66, 56, 138])
    );
}

#[test]
fn regression3() {
    // A upper shift shift 3 character for C40
    forth_and_back(&[32, 74, 224, 245]);
    assert_eq!(
        decode_data(&[230, 22, 90, 187, 209, 254, 235, 118]),
        Ok(vec![32, 74, 224, 245])
    );
}

#[test]
fn regression4() {
    // A single UNLATCH at the end of data
    forth_and_back(&[10, 39, 66, 66, 138]);
    assert_eq!(
        decode_data(&[11, 40, 230, 96, 26, 187, 139, 254]),
        Ok(vec![10, 39, 66, 66, 138])
    );
}

#[test]
fn regression5() {
    forth_and_back(&[32, 32, 153, 205]);
    assert_eq!(
        decode_data(&[239, 19, 58, 187, 154, 10, 243, 254, 235, 78]),
        Ok(vec![32, 32, 153, 205])
    );
}

#[test]
fn regression6() {
    // A UNLATCH at the end of data (X12) mode
    forth_and_back(&[43, 4, 32, 32, 32, 74, 32, 32]);
    assert_eq!(
        decode_data(&[44, 5, 238, 19, 60, 144, 60, 254]),
        Ok(vec![43, 4, 32, 32, 32, 74, 32, 32])
    );
}

#[test]
fn regression7() {
    forth_and_back(&[42, 32, 56, 40, 68, 68, 68, 68, 68, 74, 167]);
    assert_eq!(
        decode_data(&[43, 33, 57, 41, 238, 108, 250, 109, 0, 254, 235, 40]),
        Ok(vec![42, 32, 56, 40, 68, 68, 68, 68, 68, 74, 167])
    );
}

#[test]
fn regression8() {
    forth_and_back(&[255, 74, 66, 57, 32, 50, 74, 255]);
    assert_eq!(
        decode_data(&[235, 128, 238, 146, 38, 19, 200, 254, 235, 128]),
        Ok(vec![255, 74, 66, 57, 32, 50, 74, 255])
    );
}

#[test]
fn regression9() {
    forth_and_back(&[
        35, 137, 205, 74, 204, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 116, 116, 116,
        116, 116, 116, 116, 116, 116, 116, 116, 116, 116, 116, 116, 116, 255, 255, 255, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 74, 0, 9, 0, 48, 32, 10, 32, 57, 57,
    ]);
}

#[test]
fn regression10() {
    // This one was very slow, planner did consider too many options
    forth_and_back(&[
        63, 32, 32, 37, 32, 32, 32, 32, 32, 32, 1, 0, 185, 185, 185, 185, 185, 185, 185, 185, 185,
        185, 185, 185, 185, 185, 185, 185, 185, 185, 185, 185, 185, 185, 0, 0, 0, 0, 0, 32, 32, 72,
        0, 32, 32, 32, 32, 32, 32, 32, 32, 32, 255, 32, 218, 32, 32, 16,
    ]);
}

#[test]
fn regression11() {
    // A decoder bug in EDIFACT for end of data situation
    forth_and_back(&[64, 75, 75, 75, 75, 61, 75, 32, 126]);
    assert_eq!(
        decode_data(&[240, 0, 178, 203, 47, 210, 224, 127]),
        Ok(vec![64, 75, 75, 75, 75, 61, 75, 32, 126]),
    );
}

#[test]
fn regression12() {
    // EDIFACT encoding bug, after last triple written no data left, but
    // <= 2 symbol space, so unlatch not required
    forth_and_back(&[48, 47, 47, 48, 47, 47, 64, 93]);
    assert_eq!(
        decode_data(&[240, 194, 251, 240, 190, 240, 29, 129]),
        Ok(vec![48, 47, 47, 48, 47, 47, 64, 93]),
    );
}

#[test]
fn regression13() {
    // An off-by-one error in the Base256 decoding for >= 250 data len case
    let input = vec![
        205, 205, 126, 64, 215, 215, 215, 234, 234, 234, 234, 234, 234, 234, 234, 234, 234, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
        215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215, 215,
    ];
    forth_and_back(&input);
    assert_eq!(
        decode_data(&[
            231, 38, 195, 36, 185, 0, 88, 132, 26, 175, 88, 238, 131, 25, 174, 68, 217, 111, 5,
            154, 29, 178, 72, 222, 115, 9, 158, 52, 201, 95, 245, 138, 32, 181, 75, 225, 118, 12,
            161, 55, 204, 98, 248, 141, 35, 184, 78, 228, 121, 15, 164, 58, 207, 101, 251, 144, 38,
            187, 81, 231, 124, 18, 167, 61, 210, 104, 254, 147, 41, 190, 84, 234, 127, 21, 170, 64,
            213, 107, 1, 150, 44, 193, 87, 237, 130, 24, 173, 67, 217, 110, 4, 153, 47, 196, 90,
            240, 133, 27, 176, 70, 220, 113, 7, 156, 50, 199, 93, 243, 136, 30, 179, 73, 223, 116,
            10, 159, 53, 202, 96, 246, 139, 33, 182, 76, 226, 119, 13, 162, 56, 205, 99, 249, 142,
            36, 185, 79, 229, 122, 16, 165, 59, 208, 102, 252, 145, 39, 188, 82, 232, 125, 19, 168,
            62, 211, 105, 255, 148, 42, 191, 85, 235, 128, 22, 171, 65, 214, 108, 2, 151, 45, 194,
            88, 238, 131, 25, 174, 68, 218, 111, 5, 154, 48, 197, 91, 241, 134, 28, 177, 71, 221,
            114, 8, 157, 51, 200, 94, 28, 177, 71, 220, 114, 8, 157, 51, 200, 94, 243, 137, 31,
            180, 74, 223, 117, 11, 160, 54, 203, 97, 206, 100, 250, 143, 37, 186, 80, 230, 123, 17,
            166, 60, 209, 103, 253, 146, 40, 189, 83, 233, 126, 20, 169, 63, 212, 106, 0, 149, 43,
            192, 86, 236, 129, 23, 172, 66, 216, 129, 220, 115, 11, 161, 56, 206, 101, 251, 147,
            42, 192, 87, 237, 133, 28, 178, 73, 223, 118, 14, 164, 59, 209, 104
        ]),
        Ok(input),
    );
}

#[test]
fn regression14() {
    // This one creates a interesting end of data situation during planning
    let input = vec![
        75, 89, 91, 91, 91, 77, 37, 89, 91, 75, 216, 75, 75, 37, 91, 75, 91, 91, 91, 91, 91, 75,
        91, 75, 75, 91, 42, 75, 91, 137, 75, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 0, 0, 0, 0, 23, 91, 137, 75, 91, 91, 91, 91, 91, 75, 91,
        75, 75, 91, 42, 75, 91, 137, 75, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 137, 75, 91, 91, 91, 91, 75, 59, 91, 67, 75, 0, 0, 0, 0, 0, 0, 0, 23, 91,
        137, 75, 91, 91, 91, 91, 91, 75, 91, 75, 75, 91, 42, 75, 91, 137, 75, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 91, 91, 91, 91, 75, 75,
        91, 75, 91, 91, 75, 91, 75, 91, 58, 58, 58, 48, 32, 32, 74, 56, 241, 91, 91, 75, 91, 75,
        75, 91, 42, 75, 91, 137, 75, 91, 91, 91, 91, 75, 59, 91, 67, 75, 0, 0, 0, 0, 0, 0, 0, 23,
        91, 137, 75, 91, 91, 91, 91, 91, 75, 91, 75, 75, 91, 42, 75, 91, 137, 75, 91, 91, 91, 91,
        75, 75, 91, 75, 91, 95, 137, 58, 58, 91, 75, 91, 75, 75, 58, 58, 91, 67, 58, 56, 58, 58,
        58, 48, 184, 241, 91, 91, 75, 75, 91, 137, 75, 91, 91, 91, 91, 75, 67, 58, 56, 58, 58, 58,
        52, 55, 241, 91, 91, 75, 91, 75, 75, 91, 42, 42, 75, 91, 137, 75, 91, 91, 91, 91, 75, 75,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 137, 75, 91, 91, 91, 91, 75, 59, 91, 67, 75, 0, 0, 0, 0, 0, 0, 0, 23, 91, 137,
        75, 91, 91, 91, 91, 91, 75, 91, 75, 75, 91, 42, 75, 91, 137, 75, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 0, 0, 0, 0, 23, 91, 137,
        75, 91, 91, 91, 91, 91, 75, 91, 75, 75, 91, 42, 75, 91, 137, 75, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 137, 75, 91, 91, 91, 91, 75, 59, 91, 67, 75,
        0, 0, 0, 0, 0, 0, 0, 23, 91, 137, 75, 91, 91, 91, 91, 91, 75, 91, 75, 75, 91, 42, 75, 91,
        137, 75, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145, 145,
        145, 91, 91, 91, 91, 75, 75, 91, 75, 91, 95, 137, 58, 58, 91, 75, 91, 75, 75, 58, 58, 91,
        67, 58, 56, 58, 58, 58, 48, 56, 241, 91, 91, 75, 75, 91, 137, 75, 91, 91, 91, 91, 75, 67,
        58, 56, 58, 58, 58, 52, 55, 241, 91, 91, 75, 91, 75, 75, 91, 42, 42, 75, 91, 0, 0, 0, 0, 0,
        0, 0, 23, 91, 137, 0, 0, 0, 0, 0, 0, 0, 23, 91, 137, 75, 91, 91, 91, 75, 91, 46, 75, 75,
        75, 75, 75, 140, 77, 37, 91, 75, 91, 89, 91,
    ];
    forth_and_back(&input);
}

#[test]
fn regression15() {
    let input = &include_bytes!("input1.raw")[..];
    forth_and_back(input);
}

#[test]
fn regression16() {
    forth_and_back(&[32, 64, 255, 83, 48, 76, 63, 20]);
}

#[test]
fn regression17() {
    forth_and_back(&[108, 72, 72, 58, 72, 72, 72]);
}

#[test]
fn regression18() {
    forth_and_back(b"\xa3_>>>>> \x82");
}

#[test]
fn regression19() {
    // C40 encoding did not match planner behavior
    forth_and_back(&[
        252, 104, 104, 116, 116, 104, 104, 104, 104, 104, 104, 104, 104, 104, 104, 104, 104, 104,
        104, 104, 104, 104, 104, 216, 57, 104, 104, 140, 104, /* 24 */ 37, 77, 37, 89, 91, 91,
        91, 77, 37, 89, 91, 91, 91, 75, 91, /* 9 */ 104, 104, 75, 91, 104, 104, 104, 104, 104,
    ]);
}

#[test]
fn regression20() {
    forth_and_back(&[
        137, 65, 43, 41, 49, 214, 48, 42, 48, 32, 42, 48, 0, 0, 7, 0, 30, 30, 30, 30, 30, 30, 30,
        30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        /* 6 */ 75, 32, 32, 32, 29, 0,
    ]);
}

#[test]
fn regression21() {
    // This one uncovered a bug in text::val_size
    forth_and_back(&[
        72, 72, 1, 250, 12, 69, 0, 0, 0, // => 10 bytes ASCII
        98, 114, 98, 98, 98, 98, 205, 105, 66, 98, 98, 114, 98, 98, 66, 98, 32, // => 17 Text
        // 15,  31, 15, 15, 15, 15, 1 30 2 13,  22,  2 2, 15, 15,  31, 15, 15, 2 2, 15,  3,
        // 1            2           3      4           5            6          7         8      8*2+1 = 17
        66, 57, 57, // => 1 + 2 ASCII
        74, 74, 40, 74, 66, 64, 64, 32, 0, // 1 + 6 Edifact + '0'
    ]);
}
