use std::fmt::{Debug, Formatter, Error};
use super::{x12::is_native_x12, c40, text, EncodationType};

type C = u32;

const DENUM: C = 12;

/// Fraction with a fixed denominator.
#[derive(Copy, Clone)]
struct Frac(C);

impl Debug for Frac {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.write_fmt(format_args!("{}", self.0 as f32 / DENUM as f32))
    }
}

impl Frac {
    #[inline]
    fn new(num: C, denum: C) -> Self {
        let mut me = Self(0);
        me.add_mut(num, denum);
        me
    }

    #[inline]
    fn add_mut(&mut self, num: C, denum: C) -> &mut Self {
        debug_assert!(denum > 0 && DENUM % denum == 0);
        self.0 += num * (DENUM / denum);
        self
    }

    #[inline]
    fn add1(&self) -> Self {
        let mut new = Frac(self.0);
        new.add_mut(1, 1);
        new
    }

    #[inline]
    fn ceil(&mut self) -> &mut Self {
        let rest = self.0 % DENUM;
        if rest != 0 {
            self.0 += DENUM - rest;
        }
        self
    }
}

#[derive(Debug, Clone)]
struct Stat {
    mode: EncodationType,
    ascii: Frac,
    c40: Frac,
    text: Frac,
    x12: Frac,
    edf: Frac,
    b256: Frac,
}

impl Stat {
    fn new(mode: EncodationType) -> Self {
        let is_ascii = matches!(mode, EncodationType::Ascii);
        let mut me = if is_ascii {
            Self {
                mode,
                ascii: Frac::new(0, 1),
                c40: Frac::new(1, 1),
                text: Frac::new(1, 1),
                x12: Frac::new(1, 1),
                edf: Frac::new(1, 1),
                b256: Frac::new(5, 4),
            }
        } else {
            Self {
                mode,
                ascii: Frac::new(1, 1),
                c40: Frac::new(2, 1),
                text: Frac::new(2, 1),
                x12: Frac::new(2, 1),
                edf: Frac::new(2, 1),
                b256: Frac::new(9, 4),
            }
        };
        match mode {
            EncodationType::Ascii => (),
            EncodationType::C40 => me.c40 = Frac::new(0, 1),
            EncodationType::Text => me.text = Frac::new(0, 1),
            EncodationType::X12 => me.x12 = Frac::new(0, 1),
            EncodationType::Edifact => me.edf = Frac::new(0, 1),
            EncodationType::Base256 => me.b256 = Frac::new(0, 1),
        }
        me
    }

    #[inline]
    fn count_ascii(&mut self, ch: u8) {
        if ch.is_ascii_digit() {
            self.ascii.add_mut(1, 2);
        } else if ch > 127 {
            self.ascii.ceil().add_mut(2, 1);
        } else {
            self.ascii.ceil().add_mut(1, 1);
        }
    }

    #[inline]
    fn count_c40(&mut self, ch: u8) {
        // (1/3) * 2 per val
        self.c40.add_mut(c40::val_size(ch) as C * 2, 3);
    }

    #[inline]
    fn count_text(&mut self, ch: u8) {
        // (1/3) * 2 per val
        self.text.add_mut(text::val_size(ch) as C * 2, 3);
    }

    #[inline]
    fn count_x12(&mut self, ch: u8) {
        if is_native_x12(ch) {
            self.x12.add_mut(2, 3);
        } else if ch > 127 {
            self.x12.add_mut(13, 3);
        } else {
            self.x12.add_mut(10, 3);
        }
    }

    #[inline]
    fn count_edifact(&mut self, ch: u8) {
        if matches!(ch, 32..=94) {
            self.edf.add_mut(3, 4);
        } else if ch > 127 {
            self.edf.add_mut(17, 4);
        } else {
            self.edf.add_mut(13, 4);
        }
    }

    #[inline]
    fn count_b256(&mut self, _ch: u8) {
        // If ECI is to be implemented, this needs to be adapted
        // for FCN1, Structureed Append, Reader Programming, and Page Code handling.
        // In those case 4 is added.
        self.b256.add_mut(1, 1);
    }

    #[inline]
    fn round_up(&mut self) {
        self.ascii.ceil();
        self.c40.ceil();
        self.text.ceil();
        self.x12.ceil();
        self.edf.ceil();
        self.b256.ceil();
    }

    #[inline]
    fn min_no_ascii(&self, ch: C) -> bool {
        ch <= self.c40.0
            && ch <= self.text.0
            && ch <= self.x12.0
            && ch <= self.edf.0
            && ch <= self.b256.0
    }

    #[inline]
    fn strict_min_no_ascii(&self, ch: C) -> bool {
        ch < self.c40.0
            && ch < self.text.0
            && ch < self.x12.0
            && ch < self.edf.0
            && ch < self.b256.0
    }

    #[inline]
    fn strict_min_no_b256_no_ascii(&self, ch: C) -> bool {
        ch < self.c40.0 && ch < self.text.0 && ch < self.x12.0 && ch < self.edf.0
    }

    #[inline]
    fn strict_min_no_b256(&self, ch: C) -> bool {
        ch < self.ascii.0
            && ch < self.c40.0
            && ch < self.text.0
            && ch < self.x12.0
            && ch < self.edf.0
    }

    #[inline]
    fn strict_min_no_edf(&self, ch: C) -> bool {
        ch < self.ascii.0
            && ch < self.c40.0
            && ch < self.text.0
            && ch < self.x12.0
            && ch < self.b256.0
    }

    #[inline]
    fn strict_min_no_text(&self, ch: C) -> bool {
        ch < self.ascii.0
            && ch < self.c40.0
            && ch < self.x12.0
            && ch < self.edf.0
            && ch < self.b256.0
    }

    #[inline]
    fn strict_min_no_x12(&self, ch: C) -> bool {
        ch < self.ascii.0
            && ch < self.c40.0
            && ch < self.text.0
            && ch < self.edf.0
            && ch < self.b256.0
    }

    #[inline]
    fn strict_min_ascii_b256_edf_text(&self, ch: C) -> bool {
        ch < self.ascii.0 && ch < self.b256.0 && ch < self.edf.0 && ch < self.text.0
    }
}

fn x12_advantage(data: &[u8]) -> bool {
    for ch in data.iter() {
        if matches!(*ch, 13 | 42 | 62) {
            return true;
        }
        if !is_native_x12(*ch) {
            return false;
        }
    }
    false
}

pub(super) fn look_ahead(encodation: EncodationType, mut data: &[u8]) -> EncodationType {
    let mut stat = Stat::new(encodation);

    let mut processed = 0;
    let min_read = if encodation.is_ascii() {
        4
    } else {
        3
    };
    while let Some((ch, rest)) = data.split_first() {
        data = rest;
        let ch = *ch;

        stat.count_ascii(ch);
        stat.count_c40(ch);
        stat.count_text(ch);
        stat.count_x12(ch);
        stat.count_edifact(ch);
        stat.count_b256(ch);

        processed += 1;

        if processed >= min_read {
            let mut stat = stat.clone();
            stat.round_up();
            // is ASCII a global minimum?
            if stat.strict_min_no_ascii(stat.ascii.0) {
                return EncodationType::Ascii;
            }
            // is Base256 a global minimum (tie with ASCII allowed)
            if stat.b256.0 <= stat.ascii.0 || stat.strict_min_no_b256_no_ascii(stat.b256.0) {
                return EncodationType::Base256;
            }
            // is EDIFACT a global minimum?
            if stat.strict_min_no_edf(stat.edf.0) {
                return EncodationType::Edifact;
            }
            // is TEXT a global minimum?
            if stat.strict_min_no_text(stat.text.0) {
                return EncodationType::Text;
            }
            // is X12 a global minimum?
            if stat.strict_min_no_x12(stat.x12.0) {
                return EncodationType::X12;
            }
            if stat.strict_min_ascii_b256_edf_text(stat.c40.add1().0) {
                if stat.c40.0 < stat.x12.0 {
                    return EncodationType::C40;
                } else if stat.c40.0 == stat.x12.0 {
                    if x12_advantage(data) {
                        return EncodationType::X12;
                    } else {
                        return EncodationType::C40;
                    }
                }
            }
        }
    }
    stat.round_up();
    if stat.min_no_ascii(stat.ascii.0) {
        EncodationType::Ascii
    } else if stat.strict_min_no_b256(stat.b256.0) {
        EncodationType::Base256
    } else if stat.strict_min_no_edf(stat.edf.0) {
        EncodationType::Edifact
    } else if stat.strict_min_no_text(stat.text.0) {
        EncodationType::Text
    } else if stat.strict_min_no_x12(stat.x12.0) {
        EncodationType::X12
    } else {
        EncodationType::C40
    }
}

#[test]
fn test_frac_init() {
    assert_eq!(Frac::new(0, 1).0, 0);
    assert_eq!(Frac::new(1, 2).0, 6);
    assert_eq!(Frac::new(1, 1).0, 12);
}

#[test]
fn test_frac_add_mut() {
    assert_eq!(Frac::new(1, 2).add_mut(3, 4).0, 15);
}

#[test]
fn test_frac_add1() {
    assert_eq!(Frac::new(1, 2).add1().0, 18);
}

#[test]
fn test_frac_ceil() {
    assert_eq!(Frac::new(1, 2).ceil().0, 12);
    assert_eq!(Frac::new(12, 1).ceil().0, 12 * 12);
    assert_eq!(Frac::new(1, 1).ceil().0, 12);
    assert_eq!(Frac::new(0, 1).ceil().0, 0);
}

#[test]
fn test_edifact_switch1() {
    assert_eq!(
        look_ahead(EncodationType::Edifact, b".\xFCXX.XXX.XXX.XXX.XXX.XXX.XXX"),
        EncodationType::Ascii,
    );
    assert_eq!(
        look_ahead(EncodationType::Ascii, b".\xfcXX.XXX.XXX.XXX.XXX.XXX.XXX"),
        EncodationType::Ascii,
    );
}

#[test]
fn test_c40_text_switch1() {
    assert_eq!(
        look_ahead(EncodationType::C40, b"AIMaimaimaim"),
        EncodationType::C40,
    )
}
