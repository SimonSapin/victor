use crate::geom::physical::{Rect, Vec2};
use crate::geom::Length;
use crate::layout::fragments::{BoxFragment, Fragment};
use crate::pdf::Page;
use crate::primitives::{CssPx, Size, TextRun};

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
        match self {
            Fragment::Box(b) => b.paint_onto(page, containing_block),
            Fragment::Text(t) => {
                let mut origin = t
                    .content_rect
                    .to_physical(t.parent_style.writing_mode(), containing_block)
                    .translate(&containing_block.top_left)
                    .top_left;
                // Distance from top edge to baseline
                let ascender = t.parent_style.font.font_size * t.text.font.ascender();
                origin.y += ascender;
                page.set_color(&t.parent_style.color.color.into());
                page.show_text(&TextRun {
                    segment: &t.text,
                    font_size: t.parent_style.font.font_size.0.into(),
                    origin: origin.into(),
                })
                .unwrap();
            }
        }
    }
}

impl BoxFragment {
    fn paint_onto(&self, page: &mut Page, containing_block: &Rect<Length>) {
        let background_color = self.style.to_rgba(self.style.background.background_color);
        if background_color.alpha > 0 {
            page.set_color(&background_color.into());
            let rect = self
                .border_rect()
                .to_physical(self.style.writing_mode(), containing_block)
                .translate(&containing_block.top_left)
                .into();
            page.paint_rectangle(&rect);
        }
        let content_rect = self
            .content_rect
            .to_physical(self.style.writing_mode(), containing_block)
            .translate(&containing_block.top_left);
        for child in &self.children {
            child.paint_onto(page, &content_rect)
        }
    }
}
