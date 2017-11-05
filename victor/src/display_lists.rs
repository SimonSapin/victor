use euclid;
use fonts::Font;
use std::sync::Arc;

/// Origin at top-left corner, unit `1px`
pub struct CssPx;

pub type Length<U> = euclid::Length<f32, U>;
pub type Point<U> = euclid::TypedPoint2D<f32, U>;
pub type Size<U> = euclid::TypedSize2D<f32, U>;
pub type Rect<U> = euclid::TypedRect<f32, U>;
pub type GlyphId = Unimplemented;

pub enum Unimplemented {}

#[derive(Copy, Clone, PartialEq)]
pub struct RGBA(pub f32, pub f32, pub f32, pub f32);

pub struct Document {
    pub pages: Vec<Page>,
}

pub struct Page {
    pub size: Size<CssPx>,
    pub display_items: Vec<DisplayItem>,
}

pub enum DisplayItem {
    SolidRectangle(Rect<CssPx>, RGBA),
    Text {
        font: Arc<Font>,
        font_size: Length<CssPx>,
        color: RGBA,
        start: Point<CssPx>,
        glyphs: Vec<GlyphId>,
    },
}
