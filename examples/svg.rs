use std::fmt::Write;

use datamatrix::{
    placement::{Bitmap, PathSegment},
    SymbolSize,
};

fn bitmap_to_svg(bitmap: Bitmap<bool>) -> String {
    // SVG header and begin path at 1,1
    let mut svg: String = concat!(
        "<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\">",
        "<path fill-rule=\"evenodd\" d=\"M1,1",
    )
    .to_owned();

    // Now add the path segments which map nicely to the SVG path syntax.
    // One way to increase of decrease the size is to multiple everything
    // with a constant scale factor.
    for part in bitmap.path() {
        match part {
            PathSegment::Horizontal(n) => write!(svg, "h{}", n),
            PathSegment::Vertical(n) => write!(svg, "v{}", n),
            PathSegment::Move(dx, dy) => write!(svg, "m{},{}", dy, dx),
            PathSegment::Close => write!(svg, "z"),
        }.unwrap();
    }
    svg.push_str("\"/></svg>");
    svg
}

fn main() {
    let bitmap = datamatrix::encode(b"Hello, SVG!", SymbolSize::MinRect).unwrap();
    println!("{}", bitmap_to_svg(bitmap));
}
