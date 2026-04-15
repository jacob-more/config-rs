use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Indent(usize);

impl Indent {
    pub const fn new(level: usize) -> Self {
        Self(level)
    }
}

impl Add<usize> for Indent {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl AddAssign<usize> for Indent {
    fn add_assign(&mut self, rhs: usize) {
        self.0 = self.0.saturating_add(rhs)
    }
}

impl Add for Indent {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.add(rhs.0)
    }
}

impl AddAssign for Indent {
    fn add_assign(&mut self, rhs: Self) {
        self.add_assign(rhs.0);
    }
}

impl Sub<usize> for Indent {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl SubAssign<usize> for Indent {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 = self.0.saturating_sub(rhs);
    }
}

impl Sub for Indent {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.sub(rhs.0)
    }
}

impl SubAssign for Indent {
    fn sub_assign(&mut self, rhs: Self) {
        self.sub_assign(rhs.0);
    }
}

impl Mul<usize> for Indent {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl MulAssign<usize> for Indent {
    fn mul_assign(&mut self, rhs: usize) {
        self.0 = self.0.saturating_mul(rhs)
    }
}

impl Mul for Indent {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.mul(rhs.0)
    }
}

impl MulAssign for Indent {
    fn mul_assign(&mut self, rhs: Self) {
        self.mul_assign(rhs.0);
    }
}

impl Div<usize> for Indent {
    type Output = Self;

    fn div(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_div(rhs))
    }
}

impl DivAssign<usize> for Indent {
    fn div_assign(&mut self, rhs: usize) {
        self.0 = self.0.saturating_div(rhs)
    }
}

impl Div for Indent {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self.div(rhs.0)
    }
}

impl DivAssign for Indent {
    fn div_assign(&mut self, rhs: Self) {
        self.div_assign(rhs.0);
    }
}

impl Display for Indent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:width$}", "", width = self.0)
    }
}
