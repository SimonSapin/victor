use std::ops::Deref;
use svg::geometry::{Pair, Matrix2x2, Angle};
use svg::path::SimpleCommand;

/// Together with a starting point (typically the current point of a path),
/// this provides the *endpoint parameterization* of an elliptical arc.
///
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
#[derive(Copy, Clone, Debug)]
pub struct ByCenter {
    pub center: Pair,
    /// Non-negative
    pub radius: Pair,
    pub x_axis_rotation: Angle,
    pub start_angle: Angle,
    pub end_angle: Angle,
}

impl ByEndPoint {
    /// https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
    fn to_center_parameterization(&self, start_point: Pair) {
        let end_point = self.to;

        // Steps 2 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
        let mut rx = self.radius.x.abs();
        let mut ry = self.radius.y.abs();

        // Step 1 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
        let half_middle = (start_point - end_point) / 2.;  // (x1 - x2) / 2
        let cos = self.x_axis_rotation.cos();
        let sin = self.x_axis_rotation.sin();
        let rotation = Matrix2x2(
            cos, sin,
            -sin, cos,
        );
        let one_prime: Pair = rotation * half_middle;  // (x1', y1')

        // Step 3 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
        let ratio = Pair { x: one_prime.x / rx, y: one_prime.y / ry };
        let sum_of_square_ratios = ratio.x * ratio.x + ratio.y * ratio.y;
        if sum_of_square_ratios > 1. {
            // The ellipse was not big enough to reach from `start_point` to `end_point`.
            let root = sum_of_square_ratios.sqrt();
            rx *= root;
            ry *= root;
        }

        // Step 2 of https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
        //…

        unimplemented!();
    }

    /// Return a sequence of up to four cubic bezier curve (and in some cases line) commands
    /// that approximate the given SVG elliptical arc.
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

        let arc = self.to_center_parameterization(start_point);

        unimplemented!()
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
        self.array.as_slice().iter()
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
