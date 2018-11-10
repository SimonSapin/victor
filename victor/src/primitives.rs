use crate::text;

/// Origin at top-left corner, unit `1px`
pub struct CssPx;

pub use euclid::rect;
pub use euclid::point2 as point;
pub type Length<U> = euclid::Length<f32, U>;
pub type Point<U> = euclid::TypedPoint2D<f32, U>;
pub type Size<U> = euclid::TypedSize2D<f32, U>;
pub type Rect<U> = euclid::TypedRect<f32, U>;
pub type SideOffsets<U> = euclid::TypedSideOffsets2D<f32, U>;
pub type Scale<Src, Dest> = euclid::TypedScale<f32, Src, Dest>;

#[derive(Copy, Clone, PartialEq)]
pub struct RGBA(pub f32, pub f32, pub f32, pub f32);

pub struct TextRun {
    pub segment: text::ShapedSegment,
    pub font_size: Length<CssPx>,
    pub origin: Point<CssPx>,
}
