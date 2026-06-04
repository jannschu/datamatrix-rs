//! Render a Data Matrix to the terminal using Unicode "octant" characters,
//! packing a 2×4 pixel block into every character cell.
//!
//! Like `ascii.rs`, this reads the message from stdin:
//!
//! ```sh
//! echo -n "Hello" | cargo run --example unicode
//! ```

use datamatrix::{DataMatrix, SymbolList};
use std::io::{self, Read};

/// Return the Unicode "octant" character rendering a 2×4 pixel grid.
///
/// `bits` holds one pixel per bit, bit `i` being pixel `i` in reading order
/// (top-to-bottom, left-to-right):
///
/// ```text
///   bit0 bit1
///   bit2 bit3
///   bit4 bit5
///   bit6 bit7
/// ```
fn octant_char(bits: u8) -> char {
    if bits & 0x33 == bits >> 2 & 0x33 {
        // Quadrant cases, index by the collapsed 4-bit code.
        return [
            ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
        ][(bits & 3 | bits >> 2 & 12) as usize];
    }
    match [1, 2, 3, 20, 40, 63, 64, 128, 192, 252].binary_search(&bits) {
        // The 10 quarter / three-quarter blocks, irregular.
        Ok(i) => ['𜺨', '𜺫', '🮂', '🯦', '🯧', '🮅', '𜺣', '𜺠', '▂', '▆'][i],
        Err(below) => {
            let skip = below
                + (0..16u16)
                    .filter(|c| 80 * (c >> 2) + 5 * (c & 3) < bits as u16)
                    .count();
            char::from_u32(0x1CD00 + bits as u32 - skip as u32).unwrap()
        }
    }
}

fn main() {
    let mut buffer = vec![];
    io::stdin().read_to_end(&mut buffer).unwrap();

    let code = DataMatrix::encode(&buffer, SymbolList::default().enforce_square()).unwrap();
    let bitmap = code.bitmap();

    // Padded size, including a one-pixel quiet zone on every side. One octant
    // character spans 2×4 pixels, hence the ceiling divisions.
    let (w, h) = (bitmap.width() + 2, bitmap.height() + 2);

    // `pixels()` yields the black pixels in order (x before y), so we can walk
    // them band by band and only ever hold one row of octant cells in memory.
    let mut pixels = bitmap.pixels().peekable();
    for band in 0..h.div_ceil(4) {
        let mut line = vec![0u8; w.div_ceil(2)];
        while let Some(&(x, y)) = pixels.peek().filter(|&&(_, y)| y + 1 < 4 * (band + 1)) {
            // Shift by the quiet zone, then set the pixel's bit in its cell.
            let (px, py) = (x + 1, y + 1);
            line[px / 2] |= 1 << (2 * (py % 4) + px % 2);
            pixels.next();
        }
        let row: String = line.iter().map(|&bits| octant_char(bits)).collect();
        println!("{row}");
    }
}
