use std::fmt;
use std::ops::{Add, Sub, Mul, Div};

#[derive(Copy, Clone, PartialEq)]
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

impl Sub for Pair {
    type Output = Pair;
    fn sub(self, other: Pair) -> Pair {
        Pair { x: self.x - other.x, y: self.y - other.y }
    }
}

impl Mul<f64> for Pair {
    type Output = Pair;
    fn mul(self, factor: f64) -> Pair {
        Pair { x: self.x * factor, y: self.y * factor }
    }
}

impl Div<f64> for Pair {
    type Output = Pair;
    fn div(self, factor: f64) -> Pair {
        Pair { x: self.x / factor, y: self.y / factor }
    }
}

pub struct Matrix2x2(
    pub f64, pub f64,
    pub f64, pub f64,
);

/// With pairs being "vertical" vectors:
///
///  ( out_x )     ( m0  m1 )     ( x )
///  (       )  =  (        )  *  (   )
///  ( out_y )     ( m2  m3 )     ( y )
impl Mul<Pair> for Matrix2x2 {
    type Output = Pair;
    fn mul(self, other: Pair) -> Pair {
        Pair {
            x: self.0 * other.x + self.1 * other.y,
            y: self.2 * other.x + self.3 * other.y,
        }
    }
}
