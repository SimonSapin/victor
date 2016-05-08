use std::ops::Deref;
use svg::geometry::{Pair, Matrix2x2};
use svg::path::SimpleCommand;

#[derive(Copy, Clone, Debug)]
pub struct EllipticalArcCommand {
    /// Non-negative
    pub radius: Pair,
    pub x_axis_rotation: f64,
    pub large_arc: bool,
    pub sweep: bool,
    pub to: Pair,
}

/// Return a sequence of up to four cubic bezier curve (and in some cases line) commands
/// that approximate the given SVG elliptical arc.
///
/// All coordinates are absolute.
/// The return type dereferences to `&[SimpleCommand]`.
///
/// ```rust
/// # use victor::svg::geometry::Pair;
/// # use victor::svg::elliptical_arc::{EllipticalArcCommand, to_cubic_bezier};
/// # fn foo(current_point: Pair, arc: EllipticalArcCommand) {
/// for command in &to_cubic_bezier(current_point, &arc) {
///     println!("{:?}", command)
/// }
/// # }
/// ```
pub fn to_cubic_bezier(from: Pair, arc: &EllipticalArcCommand)
                       -> ArrayPrefix<[SimpleCommand; 4]> {
    // For unused values in the fixed-size [SimpleCommand; 4] array.
    let dummy = SimpleCommand::Line { to: arc.to };

    // https://www.w3.org/TR/SVG/implnote.html#ArcOutOfRangeParameters
    if from == arc.to {
        return ArrayPrefix { len: 0, array: [dummy, dummy, dummy, dummy] }
    }

    // Steps 1 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
    if arc.radius.x == 0. || arc.radius.y == 0. {
        // Same as `dummy`, but meaningful.
        let line = SimpleCommand::Line { to: arc.to };
        return ArrayPrefix { len: 1, array: [line, dummy, dummy, dummy] }
    }

    // Steps 2 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
    let mut rx = arc.radius.x.abs();
    let mut ry = arc.radius.y.abs();

    // (F.6.5.1) https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
    let half_middle = (from - arc.to) / 2.;  // (x1 - x2) / 2
    let cos = arc.x_axis_rotation.cos();
    let sin = arc.x_axis_rotation.sin();
    let rotation = Matrix2x2(
        cos, sin,
        -sin, cos,
    );
    let one_prime: Pair = rotation * half_middle;  // (x1', y1')

    // Step 3 of https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
    let ratio = Pair { x: one_prime.x / rx, y: one_prime.y / ry };
    let sum_of_square_ratios = ratio.x * ratio.x + ratio.y * ratio.y;
    if sum_of_square_ratios > 1. {
        // The ellipse was not big enough to reach connect `from` and `arc.to`.
        let root = sum_of_square_ratios.sqrt();
        rx *= root;
        ry *= root;
    }

    unimplemented!()
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
