use std::ops::Deref;
use svg::geometry::{Pair, Matrix2x2, Angle, square};
use svg::path::SimpleCommand;

/// Together with a starting point (typically the current point of a path),
/// this provides the *endpoint parameterization* of an elliptical arc,
/// as it is found in SVG path syntax.
///
/// https://www.w3.org/TR/SVG/paths.html#PathDataEllipticalArcCommands
/// https://www.w3.org/TR/SVG/implnote.html#ArcSyntax
#[derive(Copy, Clone, Debug)]
pub struct ByEndPoint {
    /// Non-negative
    pub radius: Pair,
    pub x_axis_rotation: Angle,
    pub large_arc: bool,
    pub sweep: bool,
    pub to: Pair,
}

/// Provides the *center parameterization* of an elliptical arc.
///
/// https://www.w3.org/TR/SVG/implnote.html#ArcParameterizationAlternatives
///
/// Points (x, y) on the arc are:
///
/// ```notrust
/// ( x )   ( cos(φ)  -sin(φ) )   ( radius.x * cos(θ) )   ( center.x )
/// ( y ) = ( sin(φ)   cos(φ) ) * ( radius.y * sin(θ) ) + ( center.y )
/// ```
///
/// where `θ` goes from `start_angle` to `start_angle + sweep_angle`.
/// `φ` is `x_axis_rotation`.
///
/// `start_angle` and `sweep_angle` are theoretical angles, not measures of the actual arc.
///
/// An ellipse can be thought of as a circle that has been stretched along the X or Y axis
/// by the ratio of `radius.x / radius.y`, then rotated by `x_axis_rotation`.
/// `θ1` and `Δθ` and `θ + Δθ` are angles in that circle before stretching and rotating.
///
/// The arc is clockwise if `sweep_angle` is positive, counter-clockwise if it is negative.
#[derive(Copy, Clone, Debug)]
pub struct ByCenter {
    pub center: Pair,
    /// Non-negative
    pub radius: Pair,
    pub x_axis_rotation: Angle,
    pub start_angle: Angle,
    pub sweep_angle: Angle,
}

impl ByEndPoint {
    /// https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
    fn to_center_parameterization(&self, start_point: Pair) -> ByCenter {
        let end_point = self.to;

        // Steps 2 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
        let mut radius = self.radius.map(f64::abs);

        // Step 1 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
        let half_diff = (start_point - end_point) / 2.;  // (x1 - x2) / 2
        let cos = self.x_axis_rotation.cos();
        let sin = self.x_axis_rotation.sin();
        let rotation = Matrix2x2(
            cos, sin,
            -sin, cos,
        );
        let one_prime = rotation * half_diff;  // (x1', y1')

        // Step 3 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
        let mut squared_radius = radius.map(square);
        let squared_one_prime = one_prime.map(square);
        let sum_of_square_ratios = squared_one_prime.x / squared_radius.x
                                 + squared_one_prime.y / squared_radius.y;  // Λ
        if sum_of_square_ratios > 1. {
            // The ellipse was not big enough to reach from `start_point` to `end_point`.
            radius *= sum_of_square_ratios.sqrt();
            squared_radius *= sum_of_square_ratios;
        }

        // Step 2 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
        let large_numerator =
              (squared_radius.x * squared_radius.y)
            - (squared_radius.x * squared_one_prime.y)
            - (squared_radius.y * squared_one_prime.x);
        let large_denominator =
              (squared_radius.x * squared_one_prime.y)
            + (squared_radius.y + squared_one_prime.x);
        let sign = if self.large_arc == self.sweep { 1. } else { -1. };
        let factor = sign * (large_numerator / large_denominator).sqrt();
        let c_prime = Pair {
            x:  factor * radius.x * one_prime.y / radius.y,
            y: -factor * radius.y * one_prime.x / radius.y,
        };  // (cx', cy')

        // Step 3 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
        let rotation = Matrix2x2(
            cos, -sin,
            sin, cos,
        );
        let center = rotation * c_prime + (start_point + end_point) / 2.;

        // Step 4 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
        let thing = Pair {
            x: (one_prime.x - c_prime.x) / radius.x,
            y: (one_prime.y - c_prime.y) / radius.y,
        };
        let other_thing = Pair {
            x: (-one_prime.x - c_prime.x) / radius.x,
            y: (-one_prime.y - c_prime.y) / radius.y,
        };
        let start_angle = Pair { x: 1., y: 0. }.angle_to(thing);
        let mut sweep_angle = thing.angle_to(other_thing);
        if self.sweep {
            if sweep_angle.as_radians() < 0. {
                sweep_angle += Angle::from_turns(1.)
            }
        } else {
            if sweep_angle.as_radians() > 0. {
                sweep_angle -= Angle::from_turns(1.)
            }
        }

        ByCenter {
            center: center,
            radius: radius,
            x_axis_rotation: self.x_axis_rotation,
            start_angle: start_angle,
            sweep_angle: sweep_angle,
        }
    }

    /// Return a sequence of up to four cubic bezier curve (and in some cases line) commands
    /// that approximate the given SVG elliptical arc.
    ///
    /// https://www.spaceroots.org/documents/ellipse/
    ///
    /// All coordinates are absolute.
    /// The return type dereferences to `&[SimpleCommand]`.
    ///
    /// ```rust
    /// # use victor::svg::geometry::Pair;
    /// # use victor::svg::elliptical_arc::ByEndPoint;
    /// # fn foo(current_point: Pair, arc: ByEndPoint) {
    /// for command in &arc.to_cubic_bezier(current_point) {
    ///     println!("{:?}", command)
    /// }
    /// # }
    /// ```
    pub fn to_cubic_bezier(&self, start_point: Pair) -> ArrayPrefix<[SimpleCommand; 4]> {
        let end_point = self.to;

        // For unused values in the fixed-size [SimpleCommand; 4] array.
        let dummy = SimpleCommand::Line { to: end_point };

        // https://www.w3.org/TR/SVG/implnote.html#ArcOutOfRangeParameters
        if start_point == end_point {
            return ArrayPrefix { len: 0, array: [dummy, dummy, dummy, dummy] }
        }

        // Steps 1 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
        if self.radius.x == 0. || self.radius.y == 0. {
            // Same as `dummy`, but meaningful.
            let line = SimpleCommand::Line { to: end_point };
            return ArrayPrefix { len: 1, array: [line, dummy, dummy, dummy] }
        }

        let by_center = self.to_center_parameterization(start_point);

        // Bottom https://www.spaceroots.org/documents/ellipse/node22.html
        let thing = 4. + 3. * square((by_center.sweep_angle / 2.).tan());
        let alpha = by_center.sweep_angle.sin() * (thing.sqrt() - 1.) / 3.;

        let cos = self.x_axis_rotation.cos();
        let sin = self.x_axis_rotation.sin();
        let radius = by_center.radius;
        let rotation = Matrix2x2(
            -radius.x * cos, -radius.y * sin,
            -radius.x * sin,  radius.y * cos,
        );
        let derivative = |angle: Angle| rotation * Pair { x: angle.sin(), y: angle.cos() };

        let end_angle = by_center.start_angle + by_center.sweep_angle;
        let curve = SimpleCommand::Curve {
            control_1: start_point + derivative(by_center.start_angle) * alpha,
            control_2: self.to - derivative(end_angle) * alpha,
            to: self.to,
        };

        // FIXME: split up into more than one piece? How many?
        ArrayPrefix { len: 1, array: [curve, dummy, dummy, dummy] }
    }
}

/// Dereferences to a slice of the `len` first items of `array`.
/// Other items in the array are not significant.
pub struct ArrayPrefix<A> {
    /// `A` is typically an array type `[T; N]`.
    pub array: A,

    pub len: usize,
}

impl<A: Array> Deref for ArrayPrefix<A> {
    type Target = [A::Item];

    fn deref(&self) -> &[A::Item] {
        &self.array.as_slice()[..self.len]
    }
}

impl<'a, A: Array> IntoIterator for &'a ArrayPrefix<A> {
    type Item = &'a A::Item;
    type IntoIter = <&'a [A::Item] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.array.as_slice()[..self.len].iter()
    }
}

/// Implementation detail of `ArrayPrefix`.
pub trait Array {
    type Item;

    fn as_slice(&self) -> &[Self::Item];
}

impl<T> Array for [T; 4] {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self
    }
}
