//! This module contains the implementation of the GF(256) arithmetic used by
//! the Reed-Solomon codes in Data Matrix.
//!
//! The default representation of an element in GF(256) we use is given by a
//! an u8 (8bit integer) value. Its bits correspond to the coefficients of a
//! degree 7 polynomial with the least significand bit being the coefficient
//! for 1. For example:
//!
//! > 242 = 0b11110010 = x^7 + x^6 + x^5 + x^4 + x.
//!
//! Addition can be done coefficient by coefficient, as with real (the usual)
//! polynomials.
//!
//! Multiplying two polynomials can lead to powers of x higher than 7. So
//! multiplication is defined modulo a fixed polynomial. This polynomial has to be
//! chosen. Data Matrix uses the polynomial 301.
//!
//! With this choice the powers of x up to x^255, so 1, x^1, x^2, ..., x^255
//! will give us all numbers in GF(256) except for 0 (so the multiplicative sub
//! group). We say "x is a generator". This also repeats, so x^256 = 1.
//!
//! So we can identify any element
//! of GF(256), except for 0, with a power i of x. If we now want to multiply,
//! say, a and b we first lookup their powers, say, i and j. Then
//! a * b = x^i * x^j = x^(i + j). Doing the inverse lookup of x^(i + j)
//! gives us the result. These two lookup tables are called LOG and ANTI_LOG
//! in this module.
use std::ops::{Add, Div, Mul, Sub};
use std::{
    convert::{From, Into},
    ops::{DivAssign, MulAssign, Neg, SubAssign},
};
use std::{iter::Sum, ops::AddAssign};

/// Compute two lookup tables for GF(256).
const fn compute_alog_log() -> ([u8; 255], [u8; 256]) {
    let mut alog = [0u8; 255];
    let mut log = [0u8; 256];
    let mut p: u16 = 1; // polynomial representation
    let mut i: u8 = 0; // power
    while i < 255 {
        alog[i as usize] = p as u8;
        log[p as usize] = i;

        // With 0x12D as the irreducible polynomical used
        // to define multiplication, x is a primitive
        // element. So we can just compute x^i. This is was
        // happens in the next few lines. Also see the Python
        // code in extra/gf.py.
        p *= 2;
        if p >= 256 {
            p ^= 0x12D;
        }

        i += 1;
    }
    (alog, log)
}

/// Lookup table to convert element from GF(256) represented as power i of
/// a generator a to a polynomial of degree 7.
const ANTI_LOG: [u8; 255] = compute_alog_log().0;

/// Lookup table to convert an element from GF(256) represented as a degree 7 polynomial
/// to a power i for a generator a.
const LOG: [u8; 256] = compute_alog_log().1;

#[derive(Clone, Copy, PartialEq)]
pub struct GF(pub u8);

impl GF {
    // Return iterator for 1, x, x^2, x^3, ...
    pub fn primitive_powers() -> impl Iterator<Item = Self> {
        ANTI_LOG.iter().map(|x| Self(*x)).cycle()
    }

    pub fn primitive_power(i: u8) -> Self {
        GF(ANTI_LOG[i as usize])
    }

    pub fn log(self) -> usize {
        assert!(self != GF(0), "log of 0");
        LOG[self.0 as usize] as usize
    }
}

impl std::fmt::Debug for GF {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("{}₂₅₆", self.0))
    }
}

impl Add<GF> for GF {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        GF(self.0 ^ rhs.0)
    }
}

impl AddAssign<GF> for GF {
    fn add_assign(&mut self, rhs: GF) {
        *self = *self + rhs;
    }
}

impl Sub<GF> for GF {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self + rhs
    }
}

impl SubAssign<GF> for GF {
    fn sub_assign(&mut self, rhs: GF) {
        *self = *self - rhs;
    }
}

impl Mul<GF> for GF {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        if self.0 == 0 || rhs.0 == 0 {
            return GF(0);
        }
        let ia = LOG[self.0 as usize];
        let ib = LOG[rhs.0 as usize];
        let i = (ia as u16 + ib as u16) % 255;
        GF(ANTI_LOG[i as usize])
    }
}

impl Mul<usize> for GF {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self {
        // Multiplication with usize is interpretated as
        // n-times addition. Because elements are their own additive inverse
        // we only check if the numer of addition is even or odd.
        GF(self.0 * (rhs % 2) as u8)
        // Alternative with cmov, but no mul:
        // if rhs % 2 == 0 {
        //     Self(0)
        // } else {
        //     self
        // }
    }
}

impl MulAssign<GF> for GF {
    fn mul_assign(&mut self, rhs: GF) {
        *self = *self * rhs;
    }
}

impl Div<GF> for GF {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        assert_ne!(rhs.0, 0, "division by zero");
        if self.0 == 0 {
            return GF(0);
        }
        let ia = LOG[self.0 as usize];
        let ib = LOG[rhs.0 as usize];
        let mut i = ia as i16 - ib as i16;
        if i < 0 {
            i += 255;
        }
        GF(ANTI_LOG[i as usize])
    }
}

impl DivAssign<GF> for GF {
    fn div_assign(&mut self, rhs: GF) {
        *self = *self / rhs;
    }
}

impl Neg for GF {
    type Output = Self;

    fn neg(self) -> Self {
        Self(self.0)
    }
}

impl Into<u8> for GF {
    fn into(self) -> u8 {
        self.0
    }
}

impl From<u8> for GF {
    fn from(i: u8) -> Self {
        GF(i)
    }
}

impl Sum for GF {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(GF(0), |a, b| a + b)
    }
}

#[test]
fn sanity_check_tables() {
    use std::collections::HashSet;

    let anti_log: HashSet<u8> = ANTI_LOG.iter().cloned().collect();
    assert_eq!(anti_log.len(), ANTI_LOG.len());

    let log: HashSet<u8> = LOG[1..].iter().cloned().collect();
    assert_eq!(log.len(), LOG.len() - 1);

    for i in 0..255 {
        assert_eq!(i, LOG[ANTI_LOG[i] as usize] as usize);
        assert_eq!(i + 1, ANTI_LOG[LOG[i + 1] as usize] as usize);
    }
}

#[test]
fn gf256_mul() {
    assert_eq!(GF(123) * GF(1), GF(123));
    assert_eq!(GF(234) * GF(0), GF(0));
    assert_eq!(GF(0) * GF(23), GF(0));
    assert_eq!(GF(2) * GF(4) * GF(8) * GF(16) * GF(32), GF(228));
}

#[test]
fn gf256_div_mul() {
    for a in 0..=255 {
        for b in 1..=255 {
            let a_div_b = GF(a) / GF(b);
            assert_eq!(a_div_b * GF(b), GF(a));
        }
    }
}

#[test]
fn test_gf256_power_iterator() {
    let powers: Vec<GF> = GF::primitive_powers().take(500).collect();
    let mut power_direct = Vec::with_capacity(500);
    let mut a = GF(1);
    for i in 0..500 {
        power_direct.push(a);
        assert_eq!(GF::primitive_power((i % 255) as u8), a);
        a *= GF(2);
    }
    assert_eq!(powers, power_direct);
}

#[test]
fn test_neg() {
    for a in 0..255 {
        let a = GF(a);
        let ma = -a;
        assert_eq!(a + ma, GF(0), "{:?}, {:?}", a, ma);
    }
}

#[test]
fn test_mul_usize() {
    assert_eq!(GF(5) * 1, GF(5));
    assert_eq!(GF(5) * 2, GF(5) + GF(5));
}
