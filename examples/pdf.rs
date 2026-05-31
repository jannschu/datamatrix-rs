use std::io::Write;

use datamatrix::{DataMatrix, SymbolList, placement::PathSegment};
use krilla::Document;
use krilla::color::rgb;
use krilla::geom::PathBuilder;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule};

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

    // Size of one black square in PDF points (1/72 inch). Here one module is
    // 1mm wide; you could also derive this from bitmap.width()/bitmap.height()
    // and the available space.
    const SIZE: f32 = 72.0 / 25.4;

    // krilla uses a top-left origin with the y-axis pointing downwards, which
    // matches the coordinate system of Bitmap::path(), so the relative steps
    // can be applied directly. We start one module in from the top-left corner
    // to leave room for the quiet zone.
    let mut x = SIZE;
    let mut y = SIZE;
    let mut start = (x, y);

    let mut pb = PathBuilder::new();
    // The first subpath starts implicitly (path() does not emit a leading Move).
    pb.move_to(x, y);
    for segment in bitmap.path() {
        match segment {
            PathSegment::Move(dx, dy) => {
                x += SIZE * (dx as f32);
                y += SIZE * (dy as f32);
                start = (x, y);
                pb.move_to(x, y);
            }
            PathSegment::Horizontal(dx) => {
                x += SIZE * (dx as f32);
                pb.line_to(x, y);
            }
            PathSegment::Vertical(dy) => {
                y += SIZE * (dy as f32);
                pb.line_to(x, y);
            }
            PathSegment::Close => {
                pb.close();
                x = start.0;
                y = start.1;
            }
        };
    }
    let path = pb.finish().unwrap();

    // Create a PDF with a single page holding the Data Matrix and a minimal
    // quiet zone of one module around it.
    let mut document = Document::new();
    let mut page = document.start_page_with(
        PageSettings::from_wh(
            SIZE * (bitmap.width() + 2) as f32,
            SIZE * (bitmap.height() + 2) as f32,
        )
        .unwrap(),
    );
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: rgb::Color::black().into(),
        rule: FillRule::EvenOdd,
        ..Default::default()
    }));
    surface.draw_path(&path);
    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    std::io::stdout().write_all(&pdf).unwrap();
}
