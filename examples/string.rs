use datamatrix::{DataMatrix, SymbolList};

fn main() {
    let text = "Doppelgänger";
    // call `encode_str` instead of `encode` to use latin1 encoding in this case
    let enc = DataMatrix::encode_str(text, SymbolList::default().enforce_square()).unwrap();
    print!("{}", enc.bitmap().unicode());
}
