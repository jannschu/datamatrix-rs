use datamatrix::SymbolSize;

fn main() {
    let text = "Doppelgänger";
    let enc = datamatrix::encode_str(text, SymbolSize::MinSquare).unwrap();
    print!("{}", enc.unicode());
}
