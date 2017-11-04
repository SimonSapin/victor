use euclid;

/// Origin at top-left corner, unit `1px`
pub struct CssPx;

pub type Size<U> = euclid::TypedSize2D<f32, U>;
pub type Rect<U> = euclid::TypedRect<f32, U>;

#[derive(Copy, Clone, PartialEq)]
pub struct RGB(pub f32, pub f32, pub f32);

pub struct Document {
    pub pages: Vec<Page>,
}

pub struct Page {
    pub size: Size<CssPx>,
    pub display_items: Vec<DisplayItem>,
}

pub enum DisplayItem {
    SolidRectangle(Rect<CssPx>, RGB)
}
