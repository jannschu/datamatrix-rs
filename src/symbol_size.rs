use std::fmt::Debug;
use std::iter::Cloned;
use std::slice::Iter;

#[doc(hidden)]
pub trait Size: Copy + Debug {
    const DEFAULT: Self;

    fn candidates(&self) -> Cloned<Iter<Self>>;

    fn max_codeswords(&self) -> usize;

    fn max_capacity(&self) -> Capacity;

    fn num_data_codewords(&self) -> Option<usize>;

    fn symbol_for(&self, size_needed: usize) -> Option<Self> {
        self.candidates()
            .find(|s| s.num_data_codewords().unwrap() >= size_needed)
    }
}

#[doc(hidden)]
pub struct Capacity {
    pub(crate) max: usize,
    pub(crate) min: usize,
}

impl Capacity {
    pub(crate) fn new(max: usize, min: usize) -> Self {
        Self { max, min }
    }
}

pub(crate) struct BlockSetup {
    pub(crate) num_blocks: usize,
    // Number of error correction codewords per block
    pub(crate) num_ecc_per_block: usize,
}

/// The symbol sizes supported by DataMatrix.
///
/// The number behind a variant, e.g., [Square10](Self::Square10),
/// describes the number of modules (the tiny black squares) the symbol is
/// wide/tall.
#[derive(Debug, Copy, Clone, PartialEq)]
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
    /// If chosen, the library automatically picks the smallest symbol which
    /// can fit the data.
    Min,
    /// Similar to [Min](Self::Min) but limits to considered symbols only to non-square ones.
    MinRect,
    /// Similar to [Min](Self::Min) but limits to considered symbols only to square ones.
    MinSquare,
}

#[rustfmt::skip]
const SYMBOL_SIZES: [SymbolSize; 30] = [
    SymbolSize::Square10, SymbolSize::Square12, SymbolSize::Rect8x18,
    SymbolSize::Square14, SymbolSize::Rect8x32, SymbolSize::Square16,
    SymbolSize::Rect12x26, SymbolSize::Square18, SymbolSize::Square20,
    SymbolSize::Rect12x36, SymbolSize::Square22, SymbolSize::Rect16x36,
    SymbolSize::Square24, SymbolSize::Square26, SymbolSize::Rect16x48,
    SymbolSize::Square32, SymbolSize::Square36, SymbolSize::Square40,
    SymbolSize::Square44, SymbolSize::Square48, SymbolSize::Square52,
    SymbolSize::Square64, SymbolSize::Square72, SymbolSize::Square80,
    SymbolSize::Square88, SymbolSize::Square96, SymbolSize::Square104,
    SymbolSize::Square120, SymbolSize::Square132, SymbolSize::Square144,
];

#[rustfmt::skip]
const SYMBOL_SIZES_SQUARE: [SymbolSize; 24] = [
    SymbolSize::Square10, SymbolSize::Square12, SymbolSize::Square14,
    SymbolSize::Square16, SymbolSize::Square18, SymbolSize::Square20,
    SymbolSize::Square22, SymbolSize::Square24, SymbolSize::Square26,
    SymbolSize::Square32, SymbolSize::Square36, SymbolSize::Square40,
    SymbolSize::Square44, SymbolSize::Square48, SymbolSize::Square52,
    SymbolSize::Square64, SymbolSize::Square72, SymbolSize::Square80,
    SymbolSize::Square88, SymbolSize::Square96, SymbolSize::Square104,
    SymbolSize::Square120, SymbolSize::Square132, SymbolSize::Square144,
];

#[rustfmt::skip]
const SYMBOL_SIZES_RECT: [SymbolSize; 6] = [
    SymbolSize::Rect8x18, SymbolSize::Rect8x32, SymbolSize::Rect12x26,
    SymbolSize::Rect12x36, SymbolSize::Rect16x36, SymbolSize::Rect16x48,
];

impl SymbolSize {
    fn is_auto(&self) -> bool {
        matches!(self, Self::Min | Self::MinSquare | Self::MinRect)
    }
}

impl Size for SymbolSize {
    const DEFAULT: Self = SymbolSize::MinSquare;

    fn num_data_codewords(&self) -> Option<usize> {
        match self {
            Self::Square10 => Some(3),
            Self::Square12 => Some(5),
            Self::Square14 => Some(8),
            Self::Square16 => Some(12),
            Self::Square18 => Some(18),
            Self::Square20 => Some(22),
            Self::Square22 => Some(30),
            Self::Square24 => Some(36),
            Self::Square26 => Some(44),
            Self::Square32 => Some(62),
            Self::Square36 => Some(86),
            Self::Square40 => Some(114),
            Self::Square44 => Some(144),
            Self::Square48 => Some(174),
            Self::Square52 => Some(204),
            Self::Square64 => Some(280),
            Self::Square72 => Some(368),
            Self::Square80 => Some(456),
            Self::Square88 => Some(576),
            Self::Square96 => Some(696),
            Self::Square104 => Some(816),
            Self::Square120 => Some(1050),
            Self::Square132 => Some(1304),
            Self::Square144 => Some(1558),
            Self::Rect8x18 => Some(5),
            Self::Rect8x32 => Some(10),
            Self::Rect12x26 => Some(16),
            Self::Rect12x36 => Some(22),
            Self::Rect16x36 => Some(32),
            Self::Rect16x48 => Some(49),
            SymbolSize::Min | SymbolSize::MinRect | SymbolSize::MinSquare => None,
        }
    }

    fn max_capacity(&self) -> Capacity {
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
            Self::Square144 | SymbolSize::Min | SymbolSize::MinSquare => Capacity::new(3116, 1555),
            Self::Rect8x18 => Capacity::new(10, 3),
            Self::Rect8x32 => Capacity::new(20, 8),
            Self::Rect12x26 => Capacity::new(32, 14),
            Self::Rect12x36 => Capacity::new(44, 20),
            Self::Rect16x36 => Capacity::new(64, 30),
            Self::Rect16x48 | Self::MinRect => Capacity::new(98, 47),
        }
    }

    fn max_codeswords(&self) -> usize {
        if let Some(num) = self.num_data_codewords() {
            return num;
        }
        match self {
            Self::Min | Self::MinSquare => 1558,
            Self::MinRect => 49,
            _ => unreachable!(),
        }
    }

    fn candidates(&self) -> Cloned<Iter<Self>> {
        if !self.is_auto() {
            // this is probably never used, return iterator with single size
            let index = SYMBOL_SIZES
                .iter()
                .enumerate()
                .find(|(_i, size)| size == &self)
                .unwrap()
                .0;
            SYMBOL_SIZES[index..index + 1].as_ref()
        } else if matches!(self, Self::MinSquare) {
            SYMBOL_SIZES_SQUARE.as_ref()
        } else if matches!(self, Self::MinRect) {
            SYMBOL_SIZES_RECT.as_ref()
        } else {
            SYMBOL_SIZES.as_ref()
        }
        .iter()
        .cloned()
    }
}

impl SymbolSize {
    pub(crate) fn block_setup(&self) -> Option<BlockSetup> {
        match self {
            Self::Square10 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 5,
            }),
            Self::Square12 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 7,
            }),
            Self::Square14 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 10,
            }),
            Self::Square16 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 12,
            }),
            Self::Square18 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 14,
            }),
            Self::Square20 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 18,
            }),
            Self::Square22 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 20,
            }),
            Self::Square24 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 24,
            }),
            Self::Square26 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 28,
            }),
            Self::Square32 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 36,
            }),
            Self::Square36 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 42,
            }),
            Self::Square40 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 48,
            }),
            Self::Square44 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 56,
            }),
            Self::Square48 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 68,
            }),
            Self::Square52 => Some(BlockSetup {
                num_blocks: 2,
                num_ecc_per_block: 42,
            }),
            Self::Square64 => Some(BlockSetup {
                num_blocks: 2,
                num_ecc_per_block: 56,
            }),
            Self::Square72 => Some(BlockSetup {
                num_blocks: 4,
                num_ecc_per_block: 36,
            }),
            Self::Square80 => Some(BlockSetup {
                num_blocks: 4,
                num_ecc_per_block: 48,
            }),
            Self::Square88 => Some(BlockSetup {
                num_blocks: 4,
                num_ecc_per_block: 56,
            }),
            Self::Square96 => Some(BlockSetup {
                num_blocks: 4,
                num_ecc_per_block: 68,
            }),
            Self::Square104 => Some(BlockSetup {
                num_blocks: 6,
                num_ecc_per_block: 56,
            }),
            Self::Square120 => Some(BlockSetup {
                num_blocks: 6,
                num_ecc_per_block: 68,
            }),
            Self::Square132 => Some(BlockSetup {
                num_blocks: 8,
                num_ecc_per_block: 62,
            }),
            Self::Square144 => Some(BlockSetup {
                num_blocks: 10,
                num_ecc_per_block: 62,
            }),
            Self::Rect8x18 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 7,
            }),
            Self::Rect8x32 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 11,
            }),
            Self::Rect12x26 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 14,
            }),
            Self::Rect12x36 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 18,
            }),
            Self::Rect16x36 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 24,
            }),
            Self::Rect16x48 => Some(BlockSetup {
                num_blocks: 1,
                num_ecc_per_block: 28,
            }),
            SymbolSize::Min | SymbolSize::MinRect | SymbolSize::MinSquare => None,
        }
    }
}

#[test]
fn test_size_candidates_for_non_auto() {
    let all: Vec<SymbolSize> = SymbolSize::Square10.candidates().collect();
    assert_eq!(all, vec![SymbolSize::Square10]);
}

#[test]
fn test_size_candidates_auto() {
    let all: Vec<SymbolSize> = SymbolSize::Min.candidates().collect();
    let expected: Vec<SymbolSize> = SYMBOL_SIZES.into();
    assert_eq!(all, expected);
}

#[test]
fn test_size_candidates_auto_rect() {
    let all: Vec<SymbolSize> = SymbolSize::MinRect.candidates().collect();
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
    let all: Vec<SymbolSize> = SymbolSize::MinSquare.candidates().collect();
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
    for size in SYMBOL_SIZES.iter() {
        let new = size.num_data_codewords().unwrap();
        assert!(new >= last);
        last = new;
    }
}
