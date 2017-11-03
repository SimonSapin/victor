use euclid::TypedSize2D;

/// The length marker type for the PostScript point, defined as ¹⁄₇₂ of the international inch.
pub struct Pt;

pub type Size<U> = TypedSize2D<f32, U>;

pub struct Document {
    pub pages: Vec<Page>,
}

pub struct Page {
    pub size: Size<Pt>,
}
