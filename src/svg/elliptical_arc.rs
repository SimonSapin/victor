use std::ops::Deref;
use svg::geometry::Pair;
use svg::path::{SimpleCommand, EllipticalArcCommand};

/// Return a sequence of up to four cubic bezier curve (and in some cases line) commands
/// that approximate the given SVG elliptical arc.
///
/// All coordinates are absolute.
/// The return type dereferences to `&[SimpleCommand]`.
///
/// ```rust
/// # use victor::svg::geometry::Pair;
/// # use victor::svg::path::EllipticalArcCommand;
/// # use victor::svg::elliptical_arc::to_cubic_bezier;
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

    // https://www.w3.org/TR/SVG/implnote.html#ArcCorrectionOutOfRangeRadii
    if arc.radius.x == 0. || arc.radius.y == 0. {
        // Same as `dummy`, but meaningful.
        let line = SimpleCommand::Line { to: arc.to };
        return ArrayPrefix { len: 1, array: [line, dummy, dummy, dummy] }
    }
    let rx = arc.radius.x.abs();
    let ry = arc.radius.y.abs();

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
