use std::io::BufWriter;

use datamatrix::{DataMatrix, SymbolList, placement::PathSegment};
use lopdf::content::Operation;
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

    // Create PDF for only the Data Matrix and the minimal quiet zone around it
    let (doc, page1, layer1) = PdfDocument::new(
        "datamatrix example",
        SIZE * (bitmap.width() + 2) as f32,
        SIZE * (bitmap.height() + 2) as f32,
        "Layer1",
    );
    let layer = doc.get_page(page1).get_layer(layer1);
    let black = Rgb::new(0., 0., 0., None);
    layer.set_fill_color(Color::Rgb(black));

    // Construct a path starting from the top left corner.
    let mut x: Pt = SIZE.into();
    let mut y: Pt = (SIZE * (bitmap.height() + 1) as f32).into();
    layer.add_operation(Operation::new("m", vec![x.into(), y.into()]));

    // Remember last starting point
    let mut start = (x, y);
    // The PDF coordinate system is centered in the bottom left, so we
    // have to invert the relative y steps.
    let path = bitmap.path();
    for (i, segment) in path.iter().enumerate() {
        match segment {
            PathSegment::Move(dx, dy) => {
                x += (SIZE * (*dx as f32)).into();
                y -= (SIZE * (*dy as f32)).into();
                start = (x, y);
                layer.add_operation(Operation::new("m", vec![x.into(), y.into()]));
            }
            PathSegment::Horizontal(dx) => {
                x += (SIZE * (*dx as f32)).into();
                layer.add_operation(Operation::new("l", vec![x.into(), y.into()]));
            }
            PathSegment::Vertical(dy) => {
                y -= (SIZE * (*dy as f32)).into();
                layer.add_operation(Operation::new("l", vec![x.into(), y.into()]));
            }
            PathSegment::Close => {
                if i != path.len() - 1 {
                    x = start.0;
                    y = start.1;
                    layer.add_operation(Operation::new("h", vec![]));
                }
            }
        }
    }
    // Fill with "evenodd"
    layer.add_operation(Operation::new("f*", vec![]));

    doc.save(&mut BufWriter::new(std::io::stdout())).unwrap();
}
