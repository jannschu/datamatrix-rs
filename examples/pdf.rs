use std::io::BufWriter;

use datamatrix::{DataMatrix, SymbolList, placement::PathSegment};
use printpdf::*;

fn main() {
    let s = concat!(
        "Shall I compare thee to a summer's day?\n",
        "Thou art more lovely and more temperate.\n",
        "Rough winds do shake the darling buds of May,\n",
        "And summer's lease hath all too short a date.\n",
        "Sometime too hot the eye of heaven shines,\n",
        "And often is his gold complexion dimmed;\n",
        "And every fair from fair sometime declines,\n",
        "By chance, or nature's changing course, untrimmed;\n",
        "But thy eternal summer shall not fade,\n",
        "Nor lose possession of that fair thou ow'st,\n",
        "Nor shall death brag thou wand'rest in his shade,\n",
        "When in eternal lines to Time thou grow'st.\n",
        "So long as men can breathe, or eyes can see,\n",
        "So long lives this, and this gives life to thee.",
    );
    let bitmap = DataMatrix::encode(s.as_bytes(), SymbolList::default())
        .unwrap()
        .bitmap();

    // Size of one black square, you also compute this with bitmap.width(),
    // bitmap.height() and the available space.
    const SIZE: Mm = Mm(1.);

    // Construct a path starting from the top left corner.
    let mut x: Mm = SIZE;
    let mut y: Mm = SIZE * (bitmap.height() + 1) as f32;
    let black = Color::Rgb(Rgb::new(0., 0., 0., None));
    let mut ops = vec![Op::SetFillColor { col: black }];

    // Remember last starting point
    let mut start = (x, y);
    // The PDF coordinate system is centered in the bottom left, so we
    // have to invert the relative y steps.
    let mut ring_points = vec![];
    let mut rings = vec![];
    for segment in bitmap.path() {
        match segment {
            PathSegment::Move(dx, dy) => {
                x += SIZE * (dx as f32);
                y -= SIZE * (dy as f32);
                start = (x, y);
            }
            PathSegment::Horizontal(dx) => {
                x += SIZE * (dx as f32);
            }
            PathSegment::Vertical(dy) => {
                y -= SIZE * (dy as f32);
            }
            PathSegment::Close => {
                x = start.0;
                y = start.1;
            }
        };
        ring_points.push(LinePoint {
            p: Point::new(x, y),
            bezier: false,
        });
        if matches!(segment, PathSegment::Close) {
            let mut points = vec![];
            std::mem::swap(&mut ring_points, &mut points);
            rings.push(PolygonRing { points });
        }
    }
    let polygon = Polygon {
        rings,
        mode: PaintMode::Fill,
        winding_order: WindingOrder::EvenOdd,
    };
    ops.push(Op::DrawPolygon { polygon });

    // Create PDF for only the Data Matrix and the minimal quiet zone around it
    let page = PdfPage::new(
        SIZE * (bitmap.width() + 2) as f32,
        SIZE * (bitmap.height() + 2) as f32,
        ops,
    );
    let mut doc = PdfDocument::new("datamatrix example");
    doc.pages.push(page);

    doc.save_writer(
        &mut BufWriter::new(std::io::stdout()),
        &PdfSaveOptions::default(),
        &mut vec![],
    );
}
