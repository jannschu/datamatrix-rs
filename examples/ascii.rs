use datamatrix::SymbolSize;
use std::io::{self, Read};

fn main() {
    let mut buffer = vec![];
    io::stdin().read_to_end(&mut buffer);

    let enc = datamatrix::encode(&buffer, SymbolSize::MinSquare).unwrap();
    print!("{}", enc.unicode());
}
