use std::fmt;
use std::ops::{Add, Sub, Mul};

#[derive(Copy, Clone)]
pub struct Pair {
    pub x: f64,
    pub y: f64,
}

impl fmt::Debug for Pair {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        // Truncate to avoid printing values like 56.974000000000004
        write!(formatter, "({}, {})", self.x as f32, self.y as f32)
    }
}

impl Add for Pair {
    type Output = Pair;
    fn add(self, other: Pair) -> Pair {
        Pair { x: self.x + other.x, y: self.y + other.y }
    }
}

impl<'a, 'b> Add<&'b Pair> for &'a Pair {
    type Output = Pair;
    fn add(self, other: &'b Pair) -> Pair {
        *self + *other
    }
}

impl Sub for Pair {
    type Output = Pair;
    fn sub(self, other: Pair) -> Pair {
        Pair { x: self.x - other.x, y: self.y - other.y }
    }
}

impl<'a, 'b> Sub<&'b Pair> for &'a Pair {
    type Output = Pair;
    fn sub(self, other: &'b Pair) -> Pair {
        *self - *other
    }
}

impl Mul<f64> for Pair {
    type Output = Pair;
    fn mul(self, factor: f64) -> Pair {
        Pair { x: self.x * factor, y: self.y * factor }
    }
}

impl<'a> Mul<f64> for &'a Pair {
    type Output = Pair;
    fn mul(self, factor: f64) -> Pair {
        *self * factor
    }
}
