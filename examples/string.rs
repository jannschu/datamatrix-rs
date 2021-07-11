use datamatrix::{DataMatrix, SymbolList};

fn main() {
    let text = "Doppelgänger";
    let enc = DataMatrix::encode_str(text, SymbolList::default().enforce_square()).unwrap();
    print!("{}", enc.bitmap().unicode());
}
