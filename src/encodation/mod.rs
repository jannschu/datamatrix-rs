//! Implementation of the data encodation using all specified modes.
use crate::symbol_size::Size;

pub(crate) mod ascii;
mod base256;
mod c40;
pub(crate) mod edifact;
mod text;
mod x12;

mod encodation_type;
// mod look_ahead;
pub(crate) mod planner;

#[cfg(test)]
mod tests;

pub use encodation_type::EncodationType;

// The following is not implemented
// const MACRO05: u8 = 236;
// const MACRO06: u8 = 237;
// const ECI: u8 = 241;
// const FNC1: u8 = 232;
// const STRUCT_APPEND: u8 = 233;
// const READER_PROGRAMMING: u8 = 234;

pub(crate) const UNLATCH: u8 = 254;

#[derive(Debug)]
pub enum DataEncodingError {
    TooMuchData,
}

trait EncodingContext {
    /// Look ahead and switch the mode if necessary.
    ///
    /// Return `true` if the mode was switched.
    fn maybe_switch_mode(&mut self) -> Result<bool, DataEncodingError>;

    /// Compute how much space would be left in the symbol.
    ///
    /// `extra_codewords` is the number of additional codewords to be written.
    /// This number is not included in the left space. So if the symbol has
    /// two spaces left and `extra_codewords` is 1, then the function returns `Some(1)`.
    fn symbol_size_left(&mut self, extra_codewords: usize) -> Option<usize>;

    fn eat(&mut self) -> Option<u8>;

    fn backup(&mut self, steps: usize);

    fn rest(&self) -> &[u8];

    fn push(&mut self, ch: u8);

    fn replace(&mut self, index: usize, ch: u8);

    fn insert(&mut self, index: usize, ch: u8);

    /// Get the codewords written so far.
    fn codewords(&self) -> &[u8];

    fn set_mode(&mut self, mode: EncodationType);

    fn peek(&self, n: usize) -> Option<u8> {
        self.rest().get(n).cloned()
    }

    /// Number of characters yet to be encoded.
    fn characters_left(&self) -> usize {
        self.rest().len()
    }

    /// Are there more characters to process?
    fn has_more_characters(&self) -> bool {
        !self.rest().is_empty()
    }
}

pub(crate) struct GenericDataEncoder<'a, S: Size> {
    data: &'a [u8],
    input: &'a [u8],
    encodation: EncodationType,
    pub(crate) symbol_size: S,
    planned_switches: Vec<(usize, EncodationType)>,
    new_mode: Option<u8>,
    codewords: Vec<u8>,
}

impl<'a, S: Size> EncodingContext for GenericDataEncoder<'a, S> {
    fn maybe_switch_mode(&mut self) -> Result<bool, DataEncodingError> {
        let chars_left = self.characters_left();
        assert!(
            chars_left >= self.planned_switches[0].0,
            "expected to call maybe_switch_mode when {} chars left, but now {}",
            self.planned_switches[0].0,
            chars_left
        );
        let new_mode = if chars_left > 0 && chars_left == self.planned_switches[0].0 {
            let switch = self.planned_switches.remove(0);
            switch.1
        } else {
            self.encodation
        };
        let switch = new_mode != self.encodation;
        if switch {
            // switch to new mode if not ASCII
            self.set_mode(new_mode);
        }
        Ok(switch)
    }

    fn symbol_size_left(&mut self, extra_codewords: usize) -> Option<usize> {
        let size_used = self.codewords.len() + extra_codewords;
        let symbol = self.symbol_for(extra_codewords)?;
        Some(symbol.num_data_codewords().unwrap() - size_used)
    }

    fn eat(&mut self) -> Option<u8> {
        let (ch, rest) = self.data.split_first()?;
        self.data = rest;
        Some(*ch)
    }

    fn backup(&mut self, steps: usize) {
        let offset = (self.input.len() - self.data.len()) - steps;
        self.data = &self.input[offset..];
    }

    fn rest(&self) -> &[u8] {
        self.data
    }

    fn push(&mut self, ch: u8) {
        self.codewords.push(ch);
    }

    fn codewords(&self) -> &[u8] {
        &self.codewords
    }

    fn replace(&mut self, index: usize, ch: u8) {
        self.codewords[index] = ch;
    }

    fn insert(&mut self, index: usize, ch: u8) {
        self.codewords.insert(index, ch);
    }

    fn set_mode(&mut self, mode: EncodationType) {
        self.encodation = mode;
        if !mode.is_ascii() {
            self.new_mode = Some(mode.latch_from_ascii());
        }
    }
}

impl<'a, S: Size> GenericDataEncoder<'a, S> {
    pub fn with_size(data: &'a [u8], symbol_size: S) -> Self {
        Self {
            data,
            input: data,
            symbol_size,
            new_mode: None,
            encodation: EncodationType::Ascii,
            codewords: Vec::new(),
            planned_switches: vec![],
        }
    }

    pub fn write_eci(&mut self, mut c: u32) {
        self.codewords.push(ascii::ECI);
        match c {
            0..=126 => self.codewords.push(c as u8 + 1),
            127..=16382 => {
                c -= 127;
                self.codewords.push((c / 254 + 128) as u8);
                self.codewords.push((c % 254 + 1) as u8);
            },
            16383..=999999 => {
                c -= 16383;
                self.codewords.push((c / 64516 + 192) as u8);
                self.codewords.push(((c / 254) % 254 + 1) as u8);
                self.codewords.push((c % 254 + 1) as u8);
            },
            _ => panic!("illegal ECI code, bigger than 999999"),
        }
    }

    pub fn codewords(&mut self) -> Result<Vec<u8>, DataEncodingError> {
        // bigger than theoretical limit? then fail early
        if self.data.len() > self.symbol_size.max_capacity().max {
            return Err(DataEncodingError::TooMuchData);
        }

        self.codewords
            .reserve(self.upper_limit_for_number_of_codewords());

        self.planned_switches =
            planner::optimize(self.data, self.codewords.len(), EncodationType::Ascii, self.symbol_size)
                .ok_or(DataEncodingError::TooMuchData)?;

        let mut no_write_run = 0;
        while self.has_more_characters() {
            if let Some(new_mode) = self.new_mode.take() {
                self.push(new_mode);
            }
            let len = self.codewords.len();

            self.encodation.clone().encode(self)?;

            let words_written = self.codewords.len() - len;
            if words_written <= 1 {
                // no mode can do something useful in 1 word (at EOD, but that is fine)
                no_write_run += 1;
                if no_write_run > 2 {
                    panic!("no progress in encoder, this is a bug");
                }
            } else {
                no_write_run = 0;
            }
        }

        self.symbol_size = self.symbol_for(0).ok_or(DataEncodingError::TooMuchData)?;
        self.add_padding();

        let mut codewords = vec![];
        std::mem::swap(&mut codewords, &mut self.codewords);

        Ok(codewords)
    }

    fn symbol_for(&self, extra_codewords: usize) -> Option<S> {
        self.symbol_size
            .symbol_for(self.codewords.len() + extra_codewords)
    }

    fn add_padding(&mut self) {
        let mut size_left = self.symbol_size.num_data_codewords().unwrap() - self.codewords.len();
        if size_left == 0 {
            return;
        }
        if self.encodation != EncodationType::Ascii {
            self.encodation = EncodationType::Ascii;
            self.push(UNLATCH);
            size_left -= 1;
        }
        if size_left > 0 {
            self.push(ascii::PAD);
            size_left -= 1;
        }
        for _ in 0..size_left {
            // "randomize 253 state"
            let pos = self.codewords.len() + 1;
            let pseudo_random = (((149 * pos) % 253) + 1) as u16;
            let tmp = ascii::PAD as u16 + pseudo_random;
            if tmp <= 254 {
                self.push(tmp as u8);
            } else {
                self.push((tmp - 254) as u8);
            }
        }
    }

    fn upper_limit_for_number_of_codewords(&self) -> usize {
        if let Some(size) = self.symbol_size.num_data_codewords() {
            size
        } else {
            // Min case, try to find a good upper limit
            let upper_limit = self
                .symbol_size
                .candidates()
                .find(|s| {
                    // base256 encoding is the lower bound,
                    // findest smallest symbol size to hold data with base256
                    s.max_capacity().min >= self.data.len()
                })
                .map(|s| s.num_data_codewords().unwrap())
                .unwrap_or_else(|| self.symbol_size.max_codeswords());
            upper_limit
        }
    }
}


#[test]
fn test_empty() {
    let mut enc = GenericDataEncoder::with_size(&[], crate::SymbolSize::Min);
    let cw = GenericDataEncoder::codewords(&mut enc).unwrap();
    assert_eq!(cw, vec![ascii::PAD, 175, 70]);
}