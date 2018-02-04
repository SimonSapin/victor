use primitives::{Rect, Point, Size, SideOffsets};
use fonts::{Font, FontError};
use pdf::Document;
use self::css_units::*;
use std::sync::Arc;

mod css_units {
    use primitives::Scale;

    pub use primitives::CssPx as Px;
    pub struct Mm;
    pub struct In;

    impl Mm {
        pub fn per_in() -> Scale<In, Self> { Scale::new(25.4) }
    }
    impl Px {
        pub fn per_in() -> Scale<In, Self> { Scale::new(96.) }
        pub fn per_mm() -> Scale<Mm, Self> { Mm::per_in().inv() * Self::per_in() }
    }
}

pub fn layout(text: &str, font: &Arc<Font>) -> Result<Document, FontError> {
    let page_size = Size::new(210., 297.);
    let page_margin = SideOffsets::<Mm>::new_all_same(20.);
    let page = Rect::new(Point::origin(), page_size);
    let available = page.inner_rect(page_margin) * Px::per_mm();
    unimplemented!()
}
