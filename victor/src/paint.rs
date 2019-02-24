use crate::geom::physical::{Rect, Vec2};
use crate::geom::Length;
use crate::layout::fragments::Fragment;
use crate::pdf::Page;
use crate::primitives::{CssPx, Size};

impl crate::dom::Document {
    pub fn to_pdf_bytes(&self) -> Vec<u8> {
        let page_size: Size<CssPx> = Size::new(600., 800.);
        let fragments = self.layout(page_size);
        let mut doc = crate::pdf::Document::new();
        let containing_block = Rect {
            top_left: Vec2 {
                x: Length::zero(),
                y: Length::zero(),
            },
            size: Vec2 {
                x: Length {
                    px: page_size.width,
                },
                y: Length {
                    px: page_size.height,
                },
            },
        };
        {
            let mut page = doc.add_page(page_size);
            for fragment in fragments {
                fragment.paint_onto(&mut page, &containing_block)
            }
        }
        doc.write_to_pdf_bytes()
    }
}

impl Fragment {
    fn paint_onto(&self, page: &mut Page, containing_block: &Rect<Length>) {
        let background_color = self.style.to_rgba(self.style.background.background_color);
        if background_color.alpha > 0 {
            page.set_color(&background_color.into());
            page.paint_rectangle(
                &self
                    .border_rect()
                    .to_physical(self.style.writing_mode(), containing_block)
                    .into(),
            );
        }
        for child in &self.children {
            child.paint_onto(page, containing_block)
        }
    }
}
