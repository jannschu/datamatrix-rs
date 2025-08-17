use std::io::{self, Read};

use datamatrix::{DataMatrix, placement::MatrixMap};

fn main() {
    // First read a bitmap image as ASCII encoded 0s and 1s from stdin.
    // For example the following input encodes a Data Matrix:
    //
    //    1010101010
    //    1010101101
    //    1101010000
    //    1010110011
    //    1101011000
    //    1110011001
    //    1011001000
    //    1010010011
    //    1001001000
    //    1111111111
    let mut input = vec![];
    io::stdin().read_to_end(&mut input).unwrap();
    let width = input
        .iter()
        .filter(|x| matches!(*x, b'0' | b'1' | b'\n'))
        .position(|x| *x == b'\n')
        .unwrap();
    let pixels = input
        .into_iter()
        .filter_map(|b| match b {
            b'1' => Some(true),
            b'0' => Some(false),
            _ => None,
        })
        .collect::<Vec<_>>();

    let (matrix_map, size) = MatrixMap::try_from_bits(&pixels, width).unwrap();
    let data = DataMatrix::decode(&pixels, width).unwrap();
    println!("{}", matrix_map.bitmap().unicode());
    println!("Size: {:?}", size);
    println!("Content: {:?}", std::str::from_utf8(&data).unwrap());
}
