use std::fmt::Write;

use datamatrix::{
    DataMatrix, SymbolList,
    placement::{Bitmap, PathSegment},
};

fn bitmap_to_typst(bitmap: Bitmap<bool>) -> String {
    let mut img: String = String::new();
    img.push_str("#curve(\n");
    img.push_str("  fill: black,\n");
    img.push_str("  fill-rule: \"even-odd\",\n");
    img.push_str("  curve.move((1pt, 1pt)),\n");
    for part in bitmap.path() {
        match part {
            PathSegment::Horizontal(n) => {
                write!(img, "  curve.line(({n}pt, 0pt), relative: true),\n")
            }
            PathSegment::Vertical(n) => {
                write!(img, "  curve.line((0pt, {n}pt), relative: true),\n")
            }
            PathSegment::Move(dx, dy) => {
                write!(img, "  curve.move(({dx}pt, {dy}pt), relative: true),\n")
            }
            PathSegment::Close => write!(img, "  curve.close(),\n"),
        }
        .unwrap();
    }
    img.push_str(")\n");
    img
}

fn main() {
    let bitmap = DataMatrix::encode(
        b"Hello, typst!",
        SymbolList::default().enforce_rectangular(),
    )
    .unwrap()
    .bitmap();
    println!("{}", bitmap_to_typst(bitmap));
}
