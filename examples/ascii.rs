use datamatrix::{DataMatrix, SymbolList};
use std::io::{self, Read};

fn main() {
    let mut buffer = vec![];
    io::stdin().read_to_end(&mut buffer).unwrap();

    let code = DataMatrix::encode(&buffer, SymbolList::default().enforce_square()).unwrap();
    print!("{}", code.bitmap().unicode());
}
