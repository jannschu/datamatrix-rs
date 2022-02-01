use std::fmt::Write;

use datamatrix::{
    placement::{Bitmap, PathSegment},
    DataMatrix, SymbolList,
};

fn bitmap_to_eps(bitmap: Bitmap<bool>) -> String {
    // SVG header, begin path at coordinate (1, 1)
    let w = bitmap.width();
    let h = bitmap.height();
    let mut svg: String = format!(
        concat!(
            "%!PS-Adobe-2.0 EPSF-3.0\n",
            "%%BoundingBox: 0 0 {} {}\n",
            "%%EndComments\n",
            "%%BeginProlog\n",
            "4 dict begin\n",
            "/h {{ 0 rlineto }} bind def\n",
            "/v {{ 0 exch rlineto }} bind def\n",
            "/z {{ closepath }} bind def\n",
            "/m {{ rmoveto }} bind def\n",
            "%%EndProlog\n",
            "gsave\n",
            "1 {} moveto\n",
        ),
        w + 2,
        h + 2,
        h + 1,
    );
    for part in bitmap.path() {
        match part {
            PathSegment::Horizontal(n) => write!(svg, "{} h\n", n),
            PathSegment::Vertical(n) => write!(svg, "{} v\n", -n),
            PathSegment::Move(dx, dy) => write!(svg, "{} {} m\n", dx, -dy),
            PathSegment::Close => write!(svg, "z\n"),
        }
        .unwrap();
    }
    svg.push_str("eofill\ngrestore");
    svg
}

fn main() {
    let bitmap = DataMatrix::encode(b"Hello, EPS!", SymbolList::default().enforce_rectangular())
        .unwrap()
        .bitmap();
    println!("{}", bitmap_to_eps(bitmap));
}