use std::cmp::{Ord, Ordering, PartialOrd};
use std::convert::From;
use std::fmt::{Debug, Error, Formatter};
use std::ops::{Add, AddAssign, Sub, SubAssign};

pub(super) type C = u32;

const DENUM: C = 12;

/// Fraction with a fixed denominator.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) struct Frac(C);

impl Debug for Frac {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let (mut a, b) = (self.0, DENUM);
        let i = if a >= b {
            let i = a / b;
            a = a % b;
            i
        } else {
            0
        };
        if a == 0 {
            f.write_fmt(format_args!("{}", i))
        } else if i == 0 {
            f.write_fmt(format_args!("{}/{}", a, b))
        } else {
            f.write_fmt(format_args!("{} + {}/{}", i, a, b))
        }
    }
}

impl Frac {
    #[inline]
    pub fn new(num: C, denum: C) -> Self {
        let mut me = Self(0);
        me.add_mut(num, denum);
        me
    }

    #[inline]
    fn add_mut(&mut self, num: C, denum: C) {
        debug_assert!(denum > 0 && DENUM % denum == 0);
        self.0 += num * (DENUM / denum);
    }

    #[inline]
    fn sub_mut(&mut self, num: C, denum: C) {
        debug_assert!(denum > 0 && DENUM % denum == 0);
        self.0 -= num * (DENUM / denum);
    }

    #[inline]
    pub fn ceil(mut self) -> Self {
        let rest = self.0 % DENUM;
        if rest != 0 {
            self.0 += DENUM - rest;
        }
        self
    }
}

impl From<C> for Frac {
    fn from(c: C) -> Frac {
        Frac::new(c, 1)
    }
}

impl AddAssign<C> for Frac {
    fn add_assign(&mut self, rhs: C) {
        self.add_mut(rhs, 1);
    }
}

impl AddAssign for Frac {
    fn add_assign(&mut self, rhs: Frac) {
        self.add_mut(rhs.0, DENUM);
    }
}

impl SubAssign<C> for Frac {
    fn sub_assign(&mut self, rhs: C) {
        self.sub_mut(rhs, 1);
    }
}

impl SubAssign for Frac {
    fn sub_assign(&mut self, rhs: Frac) {
        self.sub_mut(rhs.0, DENUM);
    }
}

impl Add<C> for Frac {
    type Output = Self;

    fn add(mut self, rhs: C) -> Self {
        self.add_mut(rhs, 1);
        self
    }
}

impl Add for Frac {
    type Output = Self;

    fn add(mut self, rhs: Frac) -> Self {
        self.add_mut(rhs.0, DENUM);
        self
    }
}

impl Sub<C> for Frac {
    type Output = Self;

    fn sub(mut self, rhs: C) -> Self {
        self.sub_mut(rhs, 1);
        self
    }
}

impl Sub for Frac {
    type Output = Self;

    fn sub(mut self, rhs: Frac) -> Self {
        self.sub_mut(rhs.0, DENUM);
        self
    }
}

impl PartialOrd for Frac {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Frac {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[test]
fn test_order() {
    assert!(Frac::new(0, 1) < Frac::new(1, 1));
    assert!(Frac::new(1, 2) > Frac::new(1, 3));
}

#[test]
fn test_add() {
    assert_eq!(Frac::new(1, 2) + 1, Frac::new(3, 2));
    assert_eq!(Frac::new(1, 1) + 1, Frac::new(2, 1));
    assert_eq!(Frac::new(1, 1) + Frac::new(1, 2), Frac::new(3, 2));
}

#[test]
fn test_sub() {
    assert_eq!(Frac::new(3, 2) - 1, Frac::new(1, 2));
    assert_eq!(Frac::new(1, 1) - 1, Frac::new(0, 1));
    assert_eq!(Frac::new(3, 4) - Frac::new(1, 3), Frac::new(5, 12));
}

#[test]
fn test_ceil() {
    assert_eq!(Frac::new(1, 1).ceil(), Frac::new(1, 1));
    assert_eq!(Frac::new(13, 12).ceil(), Frac::new(2, 1));
}

#[test]
fn test_debug() {
    assert_eq!(format!("{:?}", Frac::new(4, 3)), "1 + 4/12");
    assert_eq!(format!("{:?}", Frac::new(3, 3)), "1");
    assert_eq!(format!("{:?}", Frac::new(3, 6)), "6/12");
}
