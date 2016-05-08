use std::f64::consts::PI;
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div};

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

impl MulAssign<f64> for Pair {
    fn mul_assign(&mut self, factor: f64) {
        self.x *= factor;
        self.y *= factor;
    }
}

impl Div<f64> for Pair {
    type Output = Pair;
    fn div(self, factor: f64) -> Pair {
        Pair { x: self.x / factor, y: self.y / factor }
    }
}

impl Pair {
    pub fn map<F: Fn(f64) -> f64>(self, f: F) -> Self {
        Pair {
            x: f(self.x),
            y: f(self.y),
        }
    }

    pub fn dot_product(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Step 4 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
    pub fn angle_to(self, other: Self) -> Angle {
        let sign = if self.x * other.y >= self.y * other.x { 1. } else { -1. };
        let squared_denominator = (square(self.x) + square(self.y))
                                * (square(other.x) + square(other.y));
        let ratio = self.dot_product(other) / squared_denominator.sqrt();
        Angle::from_radians(sign * ratio.acos())
    }
}

pub fn square(x: f64) -> f64 {
    x * x
}

pub struct Matrix2x2(
    pub f64, pub f64,
    pub f64, pub f64,
);

/// With pairs being "vertical" vectors:
///
/// ```notrust
/// ( out.x )     ( m.0  m.1 )     ( x )
/// ( out.y )  =  ( m.2  m.3 )  *  ( y )
/// ```
impl Mul<Pair> for Matrix2x2 {
    type Output = Pair;
    fn mul(self, other: Pair) -> Pair {
        Pair {
            x: self.0 * other.x + self.1 * other.y,
            y: self.2 * other.x + self.3 * other.y,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Angle {
    radians: f64,
}

impl Angle {
    pub fn from_radians(radians: f64) -> Self {
        Angle { radians: radians }
    }

    pub fn from_turns(turns: f64) -> Self {
        Angle { radians: turns * (2. * PI) }
    }

    pub fn from_degrees(degrees: f64) -> Self {
        Self::from_turns(degrees / 360.)
    }

    pub fn as_radians(self) -> f64 {
        self.radians
    }

    pub fn as_turns(self) -> f64 {
        self.radians / (2. * PI)
    }

    pub fn as_degrees(self) -> f64 {
        self.as_turns() * 360.
    }

    pub fn sin(self) -> f64 {
        self.radians.sin()
    }

    pub fn cos(self) -> f64 {
        self.radians.cos()
    }
}

impl fmt::Debug for Angle {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
         write!(formatter, "{}°", self.as_degrees() as f32)
   }
}

impl Add for Angle {
    type Output = Angle;
    fn add(self, other: Angle) -> Angle {
        Angle { radians: self.radians + other.radians }
    }
}

impl Sub for Angle {
    type Output = Angle;
    fn sub(self, other: Angle) -> Angle {
        Angle { radians: self.radians - other.radians }
    }
}

impl AddAssign for Angle {
    fn add_assign(&mut self, other: Angle) {
        self.radians += other.radians
    }
}

impl SubAssign for Angle {
    fn sub_assign(&mut self, other: Angle) {
        self.radians -= other.radians
    }
}
