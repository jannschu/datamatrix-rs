use core::cmp::{Ordering, PartialOrd};
use core::fmt::Debug;
use core::iter::{Extend, FromIterator, IntoIterator};
use core::ops::RangeBounds;

use alloc::collections::BTreeSet;

#[cfg(test)]
use alloc::{vec, vec::Vec};

#[cfg(test)]
use enum_iterator::IntoEnumIterator;

#[cfg(test)]
use pretty_assertions::assert_eq;

type SymbolCollection = BTreeSet<SymbolSize>;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Set of [symbol sizes](SymbolSize) the encoder is allowed to use.
///
/// Specifies a list of symbol sizes the encoder will pick from. The smallest
/// symbol which can hold the data is chosen.
///
/// By [default](SymbolList::default) all standard sizes defined in
/// ISO 16022 are used. The selection can be restricted to square or rectangular
/// symbols, symbols within a size range, or by giving an explicit list.
///
/// ## Examples
///
/// To get all rectangles with maximum height 20, including the rectangle extensions you can write
///
/// ```rust
/// # use datamatrix::{DataMatrix, SymbolList};
/// let code = DataMatrix::encode(
///     b"Hello, World!",
///     SymbolList::with_extended_rectangles()
///         .enforce_rectangular()
///         .enforce_height_in(..=20),
/// );
/// ```
///
/// Because [SymbolSize] and `[SymbolSize; N]` implement `Into<SymbolList>` you can write
///
/// ```rust
/// # use datamatrix::{DataMatrix, SymbolSize};
/// // a) use one specific symbol size
/// let code = DataMatrix::encode(b"content to encode", SymbolSize::Square22);
///
/// // b) custom list of allowed symbol sizes
/// let code = DataMatrix::encode(
///     b"content to encode",
///     [SymbolSize::Square22, SymbolSize::Square26],
/// );
/// ```
pub struct SymbolList {
    symbols: SymbolCollection,
}

impl SymbolList {
    /// Get standard symbol sizes extended by all [DMRE rectangles](https://e-d-c.info/projekte/dmre.html).
    ///
    /// In ISO 21471 additional rectangular sizes are defined. Be aware that
    /// your decoder might not recognize these.
    ///
    /// DMRE stands for Data Matrix Rectangular Extensions.
    pub fn with_extended_rectangles() -> Self {
        Self::with_whitelist(SYMBOL_SIZES.iter().cloned())
    }

    #[deprecated(note = "use with_extended_rectangles()")]
    #[doc(hidden)]
    pub fn with_dmre() -> Self {
        Self::with_extended_rectangles()
    }

    /// Remove all non-square symbols from the current selection.
    pub fn enforce_square(mut self) -> Self {
        self.symbols.retain(|s| s.is_square());
        self
    }

    /// Remove all square symbols from the current selection.
    pub fn enforce_rectangular(mut self) -> Self {
        self.symbols.retain(|s| !s.is_square());
        self
    }

    /// Only keep symbols with width in the given range.
    pub fn enforce_width_in<R: RangeBounds<usize>>(mut self, bounds: R) -> Self {
        self.symbols
            .retain(|s| bounds.contains(&s.block_setup().width));
        self
    }

    #[deprecated(note = "use enforce_width_in")]
    #[doc(hidden)]
    pub fn width_range(self, min_width: usize, max_width: usize) -> Self {
        if min_width <= max_width {
            self.enforce_width_in(min_width..=max_width)
        } else {
            [].into()
        }
    }

    /// Only keep symbols with height in the given range.
    pub fn enforce_height_in<R: RangeBounds<usize>>(mut self, bounds: R) -> Self {
        self.symbols
            .retain(|s| bounds.contains(&s.block_setup().height));
        self
    }

    #[deprecated(note = "use enforce_height_in")]
    #[doc(hidden)]
    pub fn height_range(self, min_height: usize, max_height: usize) -> Self {
        if min_height <= max_height {
            self.enforce_height_in(min_height..=max_height)
        } else {
            [].into()
        }
    }

    /// Create a symbol list containing only the given symbols.
    ///
    /// The list does not need to be sorted.
    ///
    /// # Panics
    ///
    /// The call panics if the slice contains more elements than symbol
    /// sizes exist.
    pub fn with_whitelist<I>(whitelist: I) -> Self
    where
        I: IntoIterator<Item = SymbolSize>,
    {
        Self::from_iter(whitelist.into_iter())
    }

    pub fn iter(&self) -> impl Iterator<Item = SymbolSize> + '_ {
        self.symbols.iter().cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Get a list with all supported symbol sizes.
    pub fn all() -> Self {
        Self::with_extended_rectangles()
    }

    /// Check if a symbol size is in this symbol list.
    pub fn contains(&self, symbol_size: &SymbolSize) -> bool {
        self.iter().any(|s| s == *symbol_size)
    }

    pub(crate) fn max_capacity(&self) -> usize {
        self.symbols
            .iter()
            .map(|s| s.capacity().max)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn first_symbol_big_enough_for(&self, size_needed: usize) -> Option<SymbolSize> {
        self.symbols
            .iter()
            .find(|s| s.num_data_codewords() >= size_needed)
            .cloned()
    }

    pub(crate) fn upper_limit_for_number_of_codewords(&self, input_len: usize) -> Option<usize> {
        if self.symbols.len() == 1 {
            self.symbols.iter().next().map(|s| s.num_data_codewords())
        } else {
            // Min case, try to find a good upper limit
            self.symbols
                .iter()
                .find(|s| {
                    // base256 encoding is the lower bound,
                    // findest smallest symbol size to hold data with base256
                    s.capacity().min >= input_len
                })
                .map(|s| s.num_data_codewords())
        }
    }
}

impl IntoIterator for SymbolList {
    type Item = SymbolSize;
    type IntoIter = <SymbolCollection as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.into_iter()
    }
}

impl FromIterator<SymbolSize> for SymbolList {
    fn from_iter<T: IntoIterator<Item = SymbolSize>>(iter: T) -> Self {
        Self {
            symbols: SymbolCollection::from_iter(iter),
        }
    }
}

impl Extend<SymbolSize> for SymbolList {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = SymbolSize>,
    {
        self.symbols.extend(iter);
    }
}

impl Default for SymbolList {
    fn default() -> Self {
        let symbols = SYMBOL_SIZES.iter().cloned().filter(|s| !s.is_dmre());
        Self::with_whitelist(symbols)
    }
}

impl From<SymbolSize> for SymbolList {
    fn from(size: SymbolSize) -> SymbolList {
        SymbolList::with_whitelist([size])
    }
}

impl<const N: usize> From<[SymbolSize; N]> for SymbolList {
    fn from(other: [SymbolSize; N]) -> SymbolList {
        SymbolList::with_whitelist(other)
    }
}

pub(crate) struct Capacity {
    /// Maximum input size a symbol can theoretically encode
    pub(crate) max: usize,
    /// Minimum input size a symbol can encode
    pub(crate) min: usize,
}

impl Capacity {
    pub(crate) fn new(max: usize, min: usize) -> Self {
        Self { max, min }
    }
}

pub(crate) struct BlockSetup {
    /// Number of interleaved error correction blocks
    pub(crate) num_ecc_blocks: usize,
    /// Number of error correction codewords per block
    pub(crate) num_ecc_per_block: usize,
    /// Total width of the symbol including alignment pattern but not quiet zone
    pub(crate) width: usize,
    /// Total height of the symbol including alignment pattern but not quiet zone
    pub(crate) height: usize,
    /// Number extra vertical separators (alignment lines)
    pub(crate) extra_vertical_alignments: usize,
    /// Number extra horizontal separators (alignment lines)
    pub(crate) extra_horizontal_alignments: usize,
}

impl BlockSetup {
    pub(crate) fn content_width(&self) -> usize {
        self.width - 2 - self.extra_vertical_alignments * 2
    }

    pub(crate) fn content_height(&self) -> usize {
        self.height - 2 - self.extra_horizontal_alignments * 2
    }
}

/// The symbol sizes supported by Data Matrix.
///
/// The number behind a variant, e.g., [Square10](SymbolSize::Square10),
/// describes the number of modules (the tiny black squares) the symbol is
/// tall/wide.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(IntoEnumIterator))]
pub enum SymbolSize {
    Square10,
    Square12,
    Square14,
    Square16,
    Square18,
    Square20,
    Square22,
    Square24,
    Square26,
    Square32,
    Square36,
    Square40,
    Square44,
    Square48,
    Square52,
    Square64,
    Square72,
    Square80,
    Square88,
    Square96,
    Square104,
    Square120,
    Square132,
    Square144,
    Rect8x18,
    Rect8x32,
    Rect12x26,
    Rect12x36,
    Rect16x36,
    Rect16x48,

    /// DMRE 8x48 variant
    Rect8x48,
    /// DMRE 8x64 variant
    Rect8x64,
    /// DMRE 8x80 variant
    Rect8x80,
    /// DMRE 8x96 variant
    Rect8x96,
    /// DMRE 8x120 variant
    Rect8x120,
    /// DMRE 8x144 variant
    Rect8x144,
    /// DMRE 12x64 variant
    Rect12x64,
    /// DMRE 12x88 variant
    Rect12x88,
    /// DMRE 16x64 variant
    Rect16x64,
    /// DMRE 20x36 variant
    Rect20x36,
    /// DMRE 20x44 variant
    Rect20x44,
    /// DMRE 20x64 variant
    Rect20x64,
    /// DMRE 22x48 variant
    Rect22x48,
    /// DMRE 24x48 variant
    Rect24x48,
    /// DMRE 24x64 variant
    Rect24x64,
    /// DMRE 26x40 variant
    Rect26x40,
    /// DMRE 26x48 variant
    Rect26x48,
    /// DMRE 26x64 variant
    Rect26x64,
}

#[rustfmt::skip]
const SYMBOL_SIZES: &[SymbolSize] = &[
    SymbolSize::Square10, SymbolSize::Square12, SymbolSize::Rect8x18, SymbolSize::Square14,
    SymbolSize::Rect8x32, SymbolSize::Square16, SymbolSize::Rect12x26, SymbolSize::Square18,
    SymbolSize::Rect8x48, SymbolSize::Square20, SymbolSize::Rect12x36, SymbolSize::Rect8x64,
    SymbolSize::Square22, SymbolSize::Rect16x36, SymbolSize::Rect8x80, SymbolSize::Square24,
    SymbolSize::Rect8x96, SymbolSize::Rect12x64, SymbolSize::Square26, SymbolSize::Rect20x36,
    SymbolSize::Rect16x48, SymbolSize::Rect8x120, SymbolSize::Rect20x44, SymbolSize::Square32,
    SymbolSize::Rect16x64, SymbolSize::Rect8x144, SymbolSize::Rect12x88, SymbolSize::Rect26x40,
    SymbolSize::Rect22x48, SymbolSize::Rect24x48, SymbolSize::Rect20x64, SymbolSize::Square36,
    SymbolSize::Rect26x48, SymbolSize::Rect24x64, SymbolSize::Square40, SymbolSize::Rect26x64,
    SymbolSize::Square44, SymbolSize::Square48, SymbolSize::Square52, SymbolSize::Square64,
    SymbolSize::Square72, SymbolSize::Square80, SymbolSize::Square88, SymbolSize::Square96,
    SymbolSize::Square104, SymbolSize::Square120, SymbolSize::Square132, SymbolSize::Square144,
];

impl SymbolSize {
    pub(crate) fn num_data_codewords(&self) -> usize {
        match self {
            Self::Square10 => 3,
            Self::Square12 => 5,
            Self::Square14 => 8,
            Self::Square16 => 12,
            Self::Square18 => 18,
            Self::Square20 => 22,
            Self::Square22 => 30,
            Self::Square24 => 36,
            Self::Square26 => 44,
            Self::Square32 => 62,
            Self::Square36 => 86,
            Self::Square40 => 114,
            Self::Square44 => 144,
            Self::Square48 => 174,
            Self::Square52 => 204,
            Self::Square64 => 280,
            Self::Square72 => 368,
            Self::Square80 => 456,
            Self::Square88 => 576,
            Self::Square96 => 696,
            Self::Square104 => 816,
            Self::Square120 => 1050,
            Self::Square132 => 1304,
            Self::Square144 => 1558,
            Self::Rect8x18 => 5,
            Self::Rect8x32 => 10,
            Self::Rect12x26 => 16,
            Self::Rect12x36 => 22,
            Self::Rect16x36 => 32,
            Self::Rect16x48 => 49,
            // DMRE
            Self::Rect8x48 => 18,
            Self::Rect8x64 => 24,
            Self::Rect8x80 => 32,
            Self::Rect8x96 => 38,
            Self::Rect8x120 => 49,
            Self::Rect8x144 => 63,
            Self::Rect12x64 => 43,
            Self::Rect12x88 => 64,
            Self::Rect16x64 => 62,
            Self::Rect20x36 => 44,
            Self::Rect20x44 => 56,
            Self::Rect20x64 => 84,
            Self::Rect22x48 => 72,
            Self::Rect24x48 => 80,
            Self::Rect24x64 => 108,
            Self::Rect26x40 => 70,
            Self::Rect26x48 => 90,
            Self::Rect26x64 => 118,
        }
    }

    pub fn is_square(&self) -> bool {
        matches!(
            self,
            Self::Square10
                | Self::Square12
                | Self::Square14
                | Self::Square16
                | Self::Square18
                | Self::Square20
                | Self::Square22
                | Self::Square24
                | Self::Square26
                | Self::Square32
                | Self::Square36
                | Self::Square40
                | Self::Square44
                | Self::Square48
                | Self::Square52
                | Self::Square64
                | Self::Square72
                | Self::Square80
                | Self::Square88
                | Self::Square96
                | Self::Square104
                | Self::Square120
                | Self::Square132
                | Self::Square144
        )
    }

    /// Symbol is part of the rectangular extension spec (ISO 21471 DMRE).
    pub fn is_dmre(&self) -> bool {
        matches!(
            self,
            Self::Rect8x48
                | Self::Rect8x64
                | Self::Rect8x80
                | Self::Rect8x96
                | Self::Rect8x120
                | Self::Rect8x144
                | Self::Rect12x64
                | Self::Rect12x88
                | Self::Rect16x64
                | Self::Rect20x36
                | Self::Rect20x44
                | Self::Rect20x64
                | Self::Rect22x48
                | Self::Rect24x48
                | Self::Rect24x64
                | Self::Rect26x40
                | Self::Rect26x48
                | Self::Rect26x64
        )
    }

    fn capacity(&self) -> Capacity {
        match self {
            Self::Square10 => Capacity::new(6, 1),
            Self::Square12 => Capacity::new(10, 3),
            Self::Square14 => Capacity::new(16, 6),
            Self::Square16 => Capacity::new(24, 10),
            Self::Square18 => Capacity::new(36, 16),
            Self::Square20 => Capacity::new(44, 20),
            Self::Square22 => Capacity::new(60, 28),
            Self::Square24 => Capacity::new(72, 34),
            Self::Square26 => Capacity::new(88, 42),
            Self::Square32 => Capacity::new(124, 60),
            Self::Square36 => Capacity::new(172, 84),
            Self::Square40 => Capacity::new(228, 112),
            Self::Square44 => Capacity::new(288, 142),
            Self::Square48 => Capacity::new(348, 172),
            Self::Square52 => Capacity::new(408, 202),
            Self::Square64 => Capacity::new(560, 277),
            Self::Square72 => Capacity::new(736, 365),
            Self::Square80 => Capacity::new(912, 453),
            Self::Square88 => Capacity::new(1152, 573),
            Self::Square96 => Capacity::new(1392, 693),
            Self::Square104 => Capacity::new(1632, 813),
            Self::Square120 => Capacity::new(2100, 1047),
            Self::Square132 => Capacity::new(2608, 1301),
            Self::Square144 => Capacity::new(3116, 1555),
            Self::Rect8x18 => Capacity::new(10, 3),
            Self::Rect8x32 => Capacity::new(20, 8),
            Self::Rect12x26 => Capacity::new(32, 14),
            Self::Rect12x36 => Capacity::new(44, 20),
            Self::Rect16x36 => Capacity::new(64, 30),
            Self::Rect16x48 => Capacity::new(98, 47),
            // DMRE
            Self::Rect8x48 => Capacity::new(36, 16),
            Self::Rect8x64 => Capacity::new(48, 22),
            Self::Rect8x80 => Capacity::new(64, 30),
            Self::Rect8x96 => Capacity::new(76, 36),
            Self::Rect8x120 => Capacity::new(98, 47),
            Self::Rect8x144 => Capacity::new(126, 61),
            Self::Rect12x64 => Capacity::new(86, 41),
            Self::Rect12x88 => Capacity::new(128, 62),
            Self::Rect16x64 => Capacity::new(124, 60),
            Self::Rect20x36 => Capacity::new(88, 42),
            Self::Rect20x44 => Capacity::new(112, 54),
            Self::Rect20x64 => Capacity::new(168, 82), // 186 in the standard, typo
            Self::Rect22x48 => Capacity::new(144, 70),
            Self::Rect24x48 => Capacity::new(160, 78),
            Self::Rect24x64 => Capacity::new(216, 106),
            Self::Rect26x40 => Capacity::new(140, 68),
            Self::Rect26x48 => Capacity::new(180, 88),
            Self::Rect26x64 => Capacity::new(236, 116),
        }
    }

    pub(crate) fn block_setup(&self) -> BlockSetup {
        match self {
            Self::Square10 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 5,
                width: 10,
                height: 10,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square12 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 7,
                width: 12,
                height: 12,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square14 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 10,
                width: 14,
                height: 14,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square16 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 12,
                width: 16,
                height: 16,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square18 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 14,
                width: 18,
                height: 18,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square20 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 18,
                width: 20,
                height: 20,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square22 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 20,
                width: 22,
                height: 22,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square24 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 24,
                width: 24,
                height: 24,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square26 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 28,
                width: 26,
                height: 26,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Square32 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 36,
                width: 32,
                height: 32,
                extra_horizontal_alignments: 1,
                extra_vertical_alignments: 1,
            },
            Self::Square36 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 42,
                width: 36,
                height: 36,
                extra_horizontal_alignments: 1,
                extra_vertical_alignments: 1,
            },
            Self::Square40 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 48,
                width: 40,
                height: 40,
                extra_horizontal_alignments: 1,
                extra_vertical_alignments: 1,
            },
            Self::Square44 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 56,
                width: 44,
                height: 44,
                extra_horizontal_alignments: 1,
                extra_vertical_alignments: 1,
            },
            Self::Square48 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 68,
                width: 48,
                height: 48,
                extra_horizontal_alignments: 1,
                extra_vertical_alignments: 1,
            },
            Self::Square52 => BlockSetup {
                num_ecc_blocks: 2,
                num_ecc_per_block: 42,
                width: 52,
                height: 52,
                extra_horizontal_alignments: 1,
                extra_vertical_alignments: 1,
            },
            Self::Square64 => BlockSetup {
                num_ecc_blocks: 2,
                num_ecc_per_block: 56,
                width: 64,
                height: 64,
                extra_horizontal_alignments: 3,
                extra_vertical_alignments: 3,
            },
            Self::Square72 => BlockSetup {
                num_ecc_blocks: 4,
                num_ecc_per_block: 36,
                width: 72,
                height: 72,
                extra_horizontal_alignments: 3,
                extra_vertical_alignments: 3,
            },
            Self::Square80 => BlockSetup {
                num_ecc_blocks: 4,
                num_ecc_per_block: 48,
                width: 80,
                height: 80,
                extra_horizontal_alignments: 3,
                extra_vertical_alignments: 3,
            },
            Self::Square88 => BlockSetup {
                num_ecc_blocks: 4,
                num_ecc_per_block: 56,
                width: 88,
                height: 88,
                extra_horizontal_alignments: 3,
                extra_vertical_alignments: 3,
            },
            Self::Square96 => BlockSetup {
                num_ecc_blocks: 4,
                num_ecc_per_block: 68,
                width: 96,
                height: 96,
                extra_horizontal_alignments: 3,
                extra_vertical_alignments: 3,
            },
            Self::Square104 => BlockSetup {
                num_ecc_blocks: 6,
                num_ecc_per_block: 56,
                width: 104,
                height: 104,
                extra_horizontal_alignments: 3,
                extra_vertical_alignments: 3,
            },
            Self::Square120 => BlockSetup {
                num_ecc_blocks: 6,
                num_ecc_per_block: 68,
                width: 120,
                height: 120,
                extra_horizontal_alignments: 5,
                extra_vertical_alignments: 5,
            },
            Self::Square132 => BlockSetup {
                num_ecc_blocks: 8,
                num_ecc_per_block: 62,
                width: 132,
                height: 132,
                extra_horizontal_alignments: 5,
                extra_vertical_alignments: 5,
            },
            Self::Square144 => BlockSetup {
                num_ecc_blocks: 10,
                num_ecc_per_block: 62,
                width: 144,
                height: 144,
                extra_horizontal_alignments: 5,
                extra_vertical_alignments: 5,
            },
            Self::Rect8x18 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 7,
                width: 18,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Rect8x32 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 11,
                width: 32,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect12x26 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 14,
                width: 26,
                height: 12,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 0,
            },
            Self::Rect12x36 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 18,
                width: 36,
                height: 12,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect16x36 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 24,
                width: 36,
                height: 16,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect16x48 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 28,
                width: 48,
                height: 16,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },

            // DMRE
            Self::Rect8x48 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 15,
                width: 48,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect8x64 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 18,
                width: 64,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect8x80 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 22,
                width: 80,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect8x96 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 28,
                width: 96,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect8x120 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 32,
                width: 120,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 5,
            },
            Self::Rect8x144 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 36,
                width: 144,
                height: 8,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 5,
            },
            Self::Rect12x64 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 27,
                width: 64,
                height: 12,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect12x88 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 36,
                width: 88,
                height: 12,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect16x64 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 36,
                width: 64,
                height: 16,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect20x36 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 28,
                width: 36,
                height: 20,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect20x44 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 34,
                width: 44,
                height: 20,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect20x64 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 42,
                width: 64,
                height: 20,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect22x48 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 38,
                width: 48,
                height: 22,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect24x48 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 41,
                width: 48,
                height: 24,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect24x64 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 46,
                width: 64,
                height: 24,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
            Self::Rect26x40 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 38,
                width: 40,
                height: 26,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect26x48 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 42,
                width: 48,
                height: 26,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 1,
            },
            Self::Rect26x64 => BlockSetup {
                num_ecc_blocks: 1,
                num_ecc_per_block: 50,
                width: 64,
                height: 26,
                extra_horizontal_alignments: 0,
                extra_vertical_alignments: 3,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn num_codewords(&self) -> usize {
        let num_data = self.num_data_codewords();
        let setup = self.block_setup();
        let num_error = setup.num_ecc_blocks * setup.num_ecc_per_block;
        num_data + num_error
    }

    pub(crate) fn has_padding_modules(&self) -> bool {
        matches!(
            self,
            Self::Square12 | Self::Square16 | Self::Square20 | Self::Square24
        )
    }
}

impl PartialOrd for SymbolSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SymbolSize {
    fn cmp(&self, other: &Self) -> Ordering {
        fn key(obj: &SymbolSize) -> (usize, usize) {
            let bs = obj.block_setup();
            (obj.num_data_codewords(), bs.width.pow(2) + bs.height.pow(2))
        }
        key(self).cmp(&key(other))
    }
}

#[test]
fn test_partial_ord_symbol_size() {
    for a in SYMBOL_SIZES {
        for b in SYMBOL_SIZES {
            assert_eq!(
                a.partial_cmp(b) == Some(core::cmp::Ordering::Equal),
                a == b,
                "a = {:?}, b = {:?}",
                a,
                b,
            );
        }
    }
}

#[test]
fn test_symbol_size_order() {
    let mut all: Vec<SymbolSize> = SYMBOL_SIZES.into();
    all.sort_unstable();
    let all2: Vec<SymbolSize> = SymbolList::all().iter().collect();
    assert_eq!(&all, &all2,);
}

#[test]
fn test_iter_all_symbols() {
    let mut all: Vec<SymbolSize> = SymbolSize::into_enum_iter().collect();
    all.sort_unstable();
    assert_eq!(&all, SYMBOL_SIZES,);
}

#[test]
fn test_size_candidates_for_non_auto() {
    let list: SymbolList = SymbolSize::Square10.into();
    let symbols: Vec<SymbolSize> = list.iter().collect();
    assert_eq!(symbols, vec![SymbolSize::Square10]);
}

#[test]
fn test_size_candidates_auto() {
    let all: Vec<SymbolSize> = SymbolList::default().iter().collect();
    let mut expected: Vec<SymbolSize> = SYMBOL_SIZES
        .iter()
        .filter(|s| !s.is_dmre())
        .cloned()
        .collect();
    expected.sort_unstable_by_key(|s| s.num_data_codewords());
    assert_eq!(all, expected);
}

#[test]
fn test_size_candidates_auto_rect() {
    let all: Vec<SymbolSize> = SymbolList::default().enforce_rectangular().iter().collect();
    let expected = vec![
        SymbolSize::Rect8x18,
        SymbolSize::Rect8x32,
        SymbolSize::Rect12x26,
        SymbolSize::Rect12x36,
        SymbolSize::Rect16x36,
        SymbolSize::Rect16x48,
    ];
    assert_eq!(all, expected);
}

#[test]
fn test_size_candidates_auto_square() {
    let all: Vec<SymbolSize> = SymbolList::default().enforce_square().iter().collect();
    let expected = vec![
        SymbolSize::Square10,
        SymbolSize::Square12,
        SymbolSize::Square14,
        SymbolSize::Square16,
        SymbolSize::Square18,
        SymbolSize::Square20,
        SymbolSize::Square22,
        SymbolSize::Square24,
        SymbolSize::Square26,
        SymbolSize::Square32,
        SymbolSize::Square36,
        SymbolSize::Square40,
        SymbolSize::Square44,
        SymbolSize::Square48,
        SymbolSize::Square52,
        SymbolSize::Square64,
        SymbolSize::Square72,
        SymbolSize::Square80,
        SymbolSize::Square88,
        SymbolSize::Square96,
        SymbolSize::Square104,
        SymbolSize::Square120,
        SymbolSize::Square132,
        SymbolSize::Square144,
    ];
    assert_eq!(all, expected);
}

#[test]
fn symbol_size_order() {
    let mut last = 0;
    for size in SymbolList::default().symbols.iter() {
        let new = size.num_data_codewords();
        assert!(new >= last);
        last = new;
    }
}

#[test]
fn test_height_range() {
    let symbols = SymbolList::with_extended_rectangles()
        .enforce_height_in(0..21)
        .symbols;
    for sym in symbols {
        assert!(sym.block_setup().height <= 20);
    }
}

#[test]
fn test_width_range() {
    let symbols = SymbolList::with_extended_rectangles()
        .enforce_width_in(9..=10)
        .symbols;
    for sym in symbols {
        assert!(sym.block_setup().width <= 10);
        assert!(sym.block_setup().width >= 9);
    }
}

#[test]
fn test_minimal_example_every_symbol() {
    use crate::DataMatrix;
    for sym in SYMBOL_SIZES {
        DataMatrix::encode(b"OK", *sym).unwrap();
    }
}

#[test]
fn test_distinquishable_by_size() {
    use alloc::collections::btree_set::BTreeSet;
    use core::iter::FromIterator;

    let sizes: Vec<_> = SYMBOL_SIZES
        .iter()
        .map(|s| {
            let setup = s.block_setup();
            (setup.width, setup.height)
        })
        .collect();
    let n = sizes.len();
    assert_eq!(n, BTreeSet::from_iter(sizes).len());
}

#[test]
fn test_list_all() {
    assert_eq!(SymbolList::all().iter().count(), SYMBOL_SIZES.len());

    for size in SymbolList::all() {
        assert!(SYMBOL_SIZES.iter().any(|s| *s == size));
    }
}

#[test]
fn test_content_sizes_consistency() {
    for size in SymbolList::all() {
        let setup = size.block_setup();
        let codewords = size.num_codewords();
        let has_padding = size.has_padding_modules();
        let padding = if has_padding { 4 } else { 0 };
        let len = codewords * 8 + padding;

        assert_eq!(len, setup.content_width() * setup.content_height());
    }
}
