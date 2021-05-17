//! Arrangement of bits in a Data Matrix symbol.
//!
//! The module contains the struct [MatrixMap] which can be used to
//! to iterate over the bit
//! positions of each codeword in the final symbol, i.e., how the black squares are
//! mapped to the encoded data as bytes. This is used to write
//! the encoded into a bitmap, and also to read it from a bitmap.
//!
//! An abstract bitmap struct [Bitmap] is the final output of encoding and the input
//! for decoding. It also contains helpers for rendering.
use alloc::{string::String, vec, vec::Vec};

use crate::symbol_size::{Size, SymbolSize};

mod path;

pub use path::PathSegment;

/// Trait for a visitor to the symbol's bits.
///
/// A bit is called "module" in the specification. Each codeword consists
/// of eight bits (modules).
///
/// During traversal the visitor is called with pointers to the
/// codewords' bits. It can either read of write them.
pub trait Visitor<B: Bit> {
    /// Visit the next codewords' bits.
    fn visit(&mut self, codeword_index: usize, bits: [&mut B; 8]);
}

/// Abstract "bit" type used in [MatrixMap].
pub trait Bit: Clone + PartialEq + core::fmt::Debug {
    const LOW: Self;
    const HIGH: Self;
}

/// Representation of the bits in a Data Matrix symbol without alignment patterns.
pub struct MatrixMap<B: Bit> {
    entries: Vec<B>,
    visited: Vec<bool>,
    width: usize,
    height: usize,
    extra_vertical_alignments: usize,
    extra_horizontal_alignments: usize,
    has_padding: bool,
}

impl<M: Bit> MatrixMap<M> {
    /// Create a new, empty matrix for the given symbol size.
    pub fn new(size: SymbolSize) -> Self {
        let num_data = size.num_data_codewords().unwrap();
        let setup = size.block_setup().unwrap();
        let num_error = setup.num_error_codes();

        let has_padding = size.has_padding_modules();
        let padding = if has_padding { 4 } else { 0 };
        let len = (num_data + num_error) * 8 + padding;
        let mut entries = Vec::with_capacity(len);
        entries.resize(len, M::LOW);

        let w = setup.width - 2 - setup.extra_vertical_alignments * 2;
        let h = setup.height - 2 - setup.extra_horizontal_alignments * 2;
        debug_assert_eq!(w * h, len);

        Self {
            entries,
            visited: vec![],
            width: w,
            height: h,
            extra_vertical_alignments: setup.extra_vertical_alignments,
            extra_horizontal_alignments: setup.extra_horizontal_alignments,
            has_padding,
        }
    }

    // Write a 4x4 padding pattern in the lower right corner if needed.
    fn write_padding(&mut self) {
        if !self.has_padding {
            return;
        }
        *self.bit_mut(self.height - 2, self.width - 2) = M::HIGH;
        *self.bit_mut(self.height - 1, self.width - 1) = M::HIGH;
    }

    /// Get the content of the matrix as a bitmap with alignment patterns added.
    pub fn bitmap(&self) -> Bitmap<M> {
        let h = self.height + 2 + 2 * self.extra_horizontal_alignments;
        let w = self.width + 2 + 2 * self.extra_vertical_alignments;
        let mut bits = vec![M::LOW; h * w];

        let idx = |i: usize, j: usize| i * w + j;

        // draw horizontal alignments
        let xtr_hor = self.extra_horizontal_alignments;
        let blk_h = (h - 2 * (xtr_hor + 1)) / (xtr_hor + 1);
        for i in 0..xtr_hor {
            let rows_before = 1 + (blk_h + 2) * i + blk_h;
            for j in 0..w {
                bits[idx(rows_before, j)] = M::HIGH;
            }
            for j in (0..w).step_by(2) {
                bits[idx(rows_before + 1, j)] = M::HIGH;
            }
        }

        // draw vertical alignments
        let xtr_ver = self.extra_vertical_alignments;
        let blk_w = (w - 2 * (xtr_ver + 1)) / (xtr_ver + 1);
        for j in 0..xtr_ver {
            let cols_before = 1 + (blk_w + 2) * j + blk_w;
            for i in 1..h {
                bits[idx(i, cols_before + 1)] = M::HIGH;
            }
            for i in (1..h).step_by(2) {
                bits[idx(i, cols_before)] = M::HIGH;
            }
        }

        for j in 0..w {
            // draw bottom alignment
            bits[idx(h - 1, j)] = M::HIGH;
        }
        for j in (0..w).step_by(2) {
            // draw top alignment
            bits[idx(0, j)] = M::HIGH;
        }
        for i in 0..h {
            // draw left alignment
            bits[idx(i, 0)] = M::HIGH;
        }
        for i in (1..h).step_by(2) {
            // draw right alignment
            bits[idx(i, w - 1)] = M::HIGH;
        }

        // copy the data
        for (b_i, b) in self.entries.iter().enumerate() {
            let mut i = b_i / self.width;
            i += 1 + (i / blk_h) * 2;
            let mut j = b_i % self.width;
            j += 1 + (j / blk_w) * 2;
            bits[idx(i, j)] = b.clone();
        }

        Bitmap { width: w, bits }
    }

    /// Traverse the symbol in codeword order and call the visitor.
    pub fn traverse<V: Visitor<M>>(&mut self, visitor: &mut V) {
        let nrow = self.height as i16;
        let ncol = self.width as i16;
        self.visited = vec![false; (nrow * ncol) as usize];

        // starting in the correct location for first character, bit 8
        let mut i = 4;
        let mut j = 0;
        let mut codeword_idx = 0;

        loop {
            // repeatedly first check for one of the special corner cases
            if i == nrow && j == 0 {
                visitor.visit(codeword_idx, self.corner1());
                codeword_idx += 1;
            }
            if i == nrow - 2 && j == 0 && ncol % 4 != 0 {
                visitor.visit(codeword_idx, self.corner2());
                codeword_idx += 1;
            }
            if i == nrow - 2 && j == 0 && ncol % 8 == 4 {
                visitor.visit(codeword_idx, self.corner3());
                codeword_idx += 1;
            }
            if i == nrow + 4 && j == 2 && ncol % 8 == 0 {
                visitor.visit(codeword_idx, self.corner4());
                codeword_idx += 1;
            }
            // sweep upward diagonally
            loop {
                if i < nrow && j >= 0 && !self.visited[(i * ncol + j) as usize] {
                    visitor.visit(codeword_idx, self.utah(i, j));
                    codeword_idx += 1;
                }
                i -= 2;
                j += 2;
                if !(i >= 0 && j < ncol) {
                    break;
                }
            }
            i += 1;
            j += 3;

            // sweep downard diagonally
            loop {
                if i >= 0 && j < ncol && !self.visited[(i * ncol + j) as usize] {
                    visitor.visit(codeword_idx, self.utah(i, j));
                    codeword_idx += 1;
                }
                i += 2;
                j -= 2;
                if !(i < nrow && j >= 0) {
                    break;
                }
            }
            i += 3;
            j += 1;

            // until entire map is traversed
            if !(i < nrow || j < ncol) {
                break;
            }
        }

        self.write_padding();
    }

    // compute idx with wrapping
    fn idx(&self, mut i: i16, mut j: i16) -> usize {
        let h = self.height as i16;
        let w = self.width as i16;
        if i < 0 {
            i += h;
            j += 4 - ((h + 4) % 8);
        }
        if j < 0 {
            j += w;
            i += 4 - ((w + 4) % 8);
        }
        (i * w + j) as usize
    }

    // compute indices for utah-shaped symbol (the standard symbol)
    fn utah(&mut self, i: i16, j: i16) -> [&mut M; 8] {
        self.bits_mut([
            self.idx(i - 2, j - 2),
            self.idx(i - 2, j - 1),
            self.idx(i - 1, j - 2),
            self.idx(i - 1, j - 1),
            self.idx(i - 1, j),
            self.idx(i, j - 2),
            self.idx(i, j - 1),
            self.idx(i, j),
        ])
    }

    fn corner1(&mut self) -> [&mut M; 8] {
        let h = self.height as i16;
        let w = self.width as i16;
        self.bits_mut([
            self.idx(h - 1, 0),
            self.idx(h - 1, 1),
            self.idx(h - 1, 2),
            self.idx(0, w - 2),
            self.idx(0, w - 1),
            self.idx(1, w - 1),
            self.idx(2, w - 1),
            self.idx(3, w - 1),
        ])
    }

    fn corner2(&mut self) -> [&mut M; 8] {
        let h = self.height as i16;
        let w = self.width as i16;
        self.bits_mut([
            self.idx(h - 3, 0),
            self.idx(h - 2, 0),
            self.idx(h - 1, 0),
            self.idx(0, w - 4),
            self.idx(0, w - 3),
            self.idx(0, w - 2),
            self.idx(0, w - 1),
            self.idx(1, w - 1),
        ])
    }

    fn corner3(&mut self) -> [&mut M; 8] {
        let h = self.height as i16;
        let w = self.width as i16;
        self.bits_mut([
            self.idx(h - 3, 0),
            self.idx(h - 2, 0),
            self.idx(h - 1, 0),
            self.idx(0, w - 2),
            self.idx(0, w - 1),
            self.idx(1, w - 1),
            self.idx(2, w - 1),
            self.idx(3, w - 1),
        ])
    }

    fn corner4(&mut self) -> [&mut M; 8] {
        let h = self.height as i16;
        let w = self.width as i16;
        self.bits_mut([
            self.idx(h - 1, 0),
            self.idx(h - 1, w - 1),
            self.idx(0, w - 3),
            self.idx(0, w - 2),
            self.idx(0, w - 1),
            self.idx(1, w - 3),
            self.idx(1, w - 2),
            self.idx(1, w - 1),
        ])
    }

    fn bit_mut(&mut self, i: usize, j: usize) -> &mut M {
        &mut self.entries[self.width * i + j]
    }

    /// Get mutable references to the indices specified in `indices`.
    fn bits_mut(&mut self, indices: [usize; 8]) -> [&mut M; 8] {
        let mut refs = [None, None, None, None, None, None, None, None];
        let mut perm: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        perm.sort_unstable_by_key(|i| indices[*i as usize]);

        let mut prev = 0;
        let mut rest: &mut [M] = &mut self.entries;
        for perm_idx in perm.iter() {
            let idx = indices[*perm_idx as usize];
            let (e, new_rest) = rest[(idx - prev)..].split_first_mut().unwrap();
            refs[*perm_idx as usize] = Some(e);
            self.visited[idx] = true;
            rest = new_rest;
            prev = idx + 1;
        }

        [
            refs[0].take().unwrap(),
            refs[1].take().unwrap(),
            refs[2].take().unwrap(),
            refs[3].take().unwrap(),
            refs[4].take().unwrap(),
            refs[5].take().unwrap(),
            refs[6].take().unwrap(),
            refs[7].take().unwrap(),
        ]
    }
}

/// An abstract bitmap.
///
/// Contains helpers for rendering the content. For rendering targets which
/// use something similar to pixels try [pixels()](Self::pixels), while
/// vector formats might profit from [path()][Self::path].
pub struct Bitmap<M> {
    width: usize,
    bits: Vec<M>,
}

impl Bit for bool {
    const LOW: bool = false;
    const HIGH: bool = true;
}

impl<B: Bit> Bitmap<B> {
    /// Return the width of the bitmap (no quiet zone included).
    pub fn width(&self) -> usize {
        self.width
    }

    /// Return the height of the bitmap (no quiet zone included).
    pub fn height(&self) -> usize {
        self.bits.len() / self.width
    }

    /// Compute a unicode representation ("ASCII art").
    ///
    /// This is intended as a demo functionality. It might look weird
    /// if the line height is wrong or if you are not using a monospaced font.
    pub fn unicode(&self) -> String {
        const BORDER: usize = 1;
        const INVERT: bool = false;
        const CHAR: [char; 4] = [' ', '▄', '▀', '█'];
        let height = self.bits.len() / self.width;
        let get = |i: usize, j: usize| -> usize {
            let res =
                if i < BORDER || i >= BORDER + height || j < BORDER || j >= BORDER + self.width {
                    B::LOW
                } else if i - BORDER < height && j - BORDER < self.width {
                    self.bits[(i - BORDER) * self.width + (j - BORDER)].clone()
                } else {
                    B::LOW
                };
            if res == B::HIGH {
                1
            } else {
                0
            }
        };
        let mut out =
            String::with_capacity((height + 2 * BORDER) * (self.width + 1 + 2 * BORDER) * 3 / 2);
        for i in (0..height + 2 * BORDER).step_by(2) {
            for j in 0..(self.width + 2 * BORDER) {
                let idx = (get(i, j) << 1) | get(i + 1, j);
                out.push(CHAR[if INVERT { (!idx) & 0b11 } else { idx }]);
            }
            out.push('\n');
        }
        out
    }

    /// Get an iterator over the "black" pixels' coordinates `(x, y)`.
    ///
    /// A black pixel refers to one of the tiny black squares a Data Matrix
    /// is usually made of. Depending on your target, such a pixel
    /// may be rendered using multiple image pixels, or whatever you use
    /// to visualize the Data Matrix.
    ///
    /// The coordinate system is centered in the top left corner starting
    /// in `(0, 0)` with a horizontal x-axis and vertical y-axis.
    /// The pixels are returned in order, incrementing x before y.
    ///
    /// A quiet zone is not included in the coordinates but one must
    /// be added when rendering: The minimum free space required around the Data Matrix
    /// has to have the width/height of one "black" pixel.
    /// The quiet zone should have the background's color.
    ///
    /// A Data Matrix can be either rendered using dark color on a light background,
    /// or the other way around. More details on contrast, size, etc. can be found in the referenced
    /// standards mentioned in the specification.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use datamatrix::SymbolSize;
    /// let bitmap = datamatrix::encode(b"Foo", SymbolSize::Square10).unwrap();
    /// for (x, y) in bitmap.pixels() {
    ///     // place square/circle at (x, y) to render this Data Matrix
    /// }
    /// ```
    pub fn pixels(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let w = self.width();
        self.bits
            .iter()
            .enumerate()
            .filter(|(_i, b)| **b == B::HIGH)
            .map(move |(i, _b)| (i % w, i / w))
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    impl super::Bit for (u16, u8) {
        const LOW: Self = (0, 0);
        const HIGH: Self = (0, 1);
    }

    struct LogVisitor;

    impl super::Visitor<(u16, u8)> for LogVisitor {
        fn visit(&mut self, cw: usize, bits: [&mut (u16, u8); 8]) {
            for i in 0..8 {
                *bits[i as usize] = ((cw + 1) as u16, (i + 1) as u8);
            }
        }
    }

    pub fn log(s: super::SymbolSize) -> Vec<(u16, u8)> {
        let mut m = super::MatrixMap::<(u16, u8)>::new(s);
        m.traverse(&mut LogVisitor);
        m.entries
    }
}

#[test]
fn test_12x12() {
    let log = tests::log(SymbolSize::Square12);
    #[rustfmt::skip]
    let should = [
        (2,1), (2,2), (3,6), (3,7), (3,8), (4,3), (4,4), (4,5), (1,1), (1,2),
        (2,3), (2,4), (2,5), (5,1), (5,2), (4,6), (4,7), (4,8), (1,3), (1,4),
        (2,6), (2,7), (2,8), (5,3), (5,4), (5,5), (10,1), (10,2), (1,6), (1,7),
        (1,5), (6,1), (6,2), (5,6), (5,7), (5,8), (10,3), (10,4), (10,5), (7,1),
        (1,8), (6,3), (6,4), (6,5), (9,1), (9,2), (10,6), (10,7), (10,8), (7,3),
        (7,2), (6,6), (6,7), (6,8), (9,3), (9,4), (9,5), (11,1), (11,2), (7,6),
        (7,4), (7,5), (8,1), (8,2), (9,6), (9,7), (9,8), (11,3), (11,4), (11,5),
        (7,7), (7,8), (8,3), (8,4), (8,5), (12,1), (12,2), (11,6), (11,7), (11,8),
        (3,1), (3,2), (8,6), (8,7), (8,8), (12,3), (12,4), (12,5), (0,1), (0,0),
        (3,3), (3,4), (3,5), (4,1), (4,2), (12,6), (12,7), (12,8), (0,0), (0,1)
    ];
    assert_eq!(&log, &should);
}

#[test]
fn test_10x10() {
    let log = tests::log(SymbolSize::Square10);
    #[rustfmt::skip]
    let should = [
        (2,1), (2,2), (3,6), (3,7), (3,8), (4,3), (4,4), (4,5),
        (2,3), (2,4), (2,5), (5,1), (5,2), (4,6), (4,7), (4,8),
        (2,6), (2,7), (2,8), (5,3), (5,4), (5,5), (1,1), (1,2),
        (1,5), (6,1), (6,2), (5,6), (5,7), (5,8), (1,3), (1,4),
        (1,8), (6,3), (6,4), (6,5), (8,1), (8,2), (1,6), (1,7),
        (7,2), (6,6), (6,7), (6,8), (8,3), (8,4), (8,5), (7,1),
        (7,4), (7,5), (3,1), (3,2), (8,6), (8,7), (8,8), (7,3),
        (7,7), (7,8), (3,3), (3,4), (3,5), (4,1), (4,2), (7,6),
    ];
    assert_eq!(&log, &should);
}

#[test]
fn test_8x32() {
    let log = tests::log(SymbolSize::Rect8x32);
    #[rustfmt::skip]
    let should = [
        (2,1), (2,2), (3,6), (3,7), (3,8), (4,3), (4,4), (4,5), (8,1), (8,2), (9,6), (9,7), (9,8), (10,3), (10,4), (10,5), (14,1), (14,2), (15,6), (15,7), (15,8), (16,3), (16,4), (16,5), (20,1), (20,2), (1,4), (1,5),
        (2,3), (2,4), (2,5), (5,1), (5,2), (4,6), (4,7), (4,8), (8,3), (8,4), (8,5), (11,1), (11,2), (10,6), (10,7), (10,8), (14,3), (14,4), (14,5), (17,1), (17,2), (16,6), (16,7), (16,8), (20,3), (20,4), (20,5), (1,6),
        (2,6), (2,7), (2,8), (5,3), (5,4), (5,5), (7,1), (7,2), (8,6), (8,7), (8,8), (11,3), (11,4), (11,5), (13,1), (13,2), (14,6), (14,7), (14,8), (17,3), (17,4), (17,5), (19,1), (19,2), (20,6), (20,7), (20,8), (1,7),
        (1,1), (6,1), (6,2), (5,6), (5,7), (5,8), (7,3), (7,4), (7,5), (12,1), (12,2), (11,6), (11,7), (11,8), (13,3), (13,4), (13,5), (18,1), (18,2), (17,6), (17,7), (17,8), (19,3), (19,4), (19,5), (21,1), (21,2), (1,8),
        (1,2), (6,3), (6,4), (6,5), (3,1), (3,2), (7,6), (7,7), (7,8), (12,3), (12,4), (12,5), (9,1), (9,2), (13,6), (13,7), (13,8), (18,3), (18,4), (18,5), (15,1), (15,2), (19,6), (19,7), (19,8), (21,3), (21,4), (21,5),
        (1,3), (6,6), (6,7), (6,8), (3,3), (3,4), (3,5), (4,1), (4,2), (12,6), (12,7), (12,8), (9,3), (9,4), (9,5), (10,1), (10,2), (18,6), (18,7), (18,8), (15,3), (15,4), (15,5), (16,1), (16,2), (21,6), (21,7), (21,8),
    ];
    assert_eq!(&log, &should);
}
