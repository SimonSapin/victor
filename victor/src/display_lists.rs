use euclid;

/// The length marker type for the PostScript point, defined as ¹⁄₇₂ of the international inch.
pub struct Pt;

pub type Size<U> = euclid::TypedSize2D<f32, U>;
pub type Rect<U> = euclid::TypedRect<f32, U>;

pub struct RGB(pub f32, pub f32, pub f32);

pub struct Document {
    pub pages: Vec<Page>,
}

pub struct Page {
    pub size: Size<Pt>,
    pub display_items: Vec<DisplayItem>,
}

pub enum DisplayItem {
    SolidRectangle(Rect<Pt>, RGB)
}
