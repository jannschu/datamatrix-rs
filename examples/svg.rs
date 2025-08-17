use std::fmt::Write;

use datamatrix::{
    DataMatrix, SymbolList,
    placement::{Bitmap, PathSegment},
};

fn bitmap_to_svg(bitmap: Bitmap<bool>) -> String {
    // SVG header, begin path at coordinate (1, 1)
    let mut svg: String = concat!(
        "<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\">",
        "<path fill-rule=\"evenodd\" d=\"M1,1",
    )
    .to_owned();

    // Now add the path segments. They map nicely to the SVG path syntax.
    // One way to increase or decrease the size is to multiply everything
    // with a constant scale factor.
    for part in bitmap.path() {
        match part {
            PathSegment::Horizontal(n) => write!(svg, "h{}", n),
            PathSegment::Vertical(n) => write!(svg, "v{}", n),
            PathSegment::Move(dx, dy) => write!(svg, "m{},{}", dx, dy),
            PathSegment::Close => write!(svg, "z"),
        }
        .unwrap();
    }
    svg.push_str("\"/></svg>");
    svg
}

fn main() {
    let bitmap = DataMatrix::encode(b"Hello, SVG!", SymbolList::default().enforce_rectangular())
        .unwrap()
        .bitmap();
    println!("{}", bitmap_to_svg(bitmap));
}
