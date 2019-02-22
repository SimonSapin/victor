use crate::layout::fragments::Fragment;
use crate::pdf::Page;
use crate::primitives::{CssPx, Size};

impl crate::dom::Document {
    pub fn to_pdf_bytes(&self) -> Vec<u8> {
        let page_size: Size<CssPx> = Size::new(600., 800.);
        let fragments = self.layout(page_size);
        let mut doc = crate::pdf::Document::new();
        {
            let mut page = doc.add_page(page_size);
            for fragment in fragments {
                fragment.paint_onto(&mut page)
            }
        }
        doc.write_to_pdf_bytes()
    }
}

impl Fragment {
    fn paint_onto(&self, page: &mut Page) {
        let background_color = self.style.to_rgba(self.style.background.background_color);
        if background_color.alpha > 0 {
            page.set_color(&background_color.into());
        }
        for child in &self.children {
            child.paint_onto(page)
        }
    }
}
