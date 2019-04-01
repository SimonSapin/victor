use self::css_units::*;
use crate::fonts::{Em, Font, FontError};
use crate::pdf::Document;
use crate::primitives::{Length, Point, Rect, SideOffsets, Size, TextRun};
use crate::text::ShapedSegment;
use std::sync::Arc;
use xi_unicode::LineBreakIterator;

pub mod css_units {
    use crate::primitives::Scale;

    pub use crate::primitives::CssPx as Px;
    pub struct Mm;
    pub struct In;

    impl Mm {
        pub fn per_in() -> Scale<In, Self> {
            Scale::new(25.4)
        }
    }
    impl Px {
        pub fn per_in() -> Scale<In, Self> {
            Scale::new(96.)
        }
        pub fn per_mm() -> Scale<Mm, Self> {
            Mm::per_in().inv() * Self::per_in()
        }
    }
}

pub struct Style {
    pub page_size: Size<Mm>,
    pub page_margin: Length<Mm>,
    pub font: Arc<Font>,
    pub font_size: Length<Px>,
    pub line_height: f32,
    pub justify: bool,
}

pub fn layout(text: &str, style: &Style) -> Result<Document, FontError> {
    let page_size = style.page_size * Px::per_mm();
    let page_margin = SideOffsets::from_length_all_same(style.page_margin * Px::per_mm());
    let page = Rect::new(Point::origin(), page_size);
    let content_area = page.inner_rect(page_margin);
    let min_x = content_area.min_x_typed();
    let min_y = content_area.min_y_typed();
    let max_y = content_area.max_y_typed();
    let available_width = content_area.size.width_typed();

    let one_em = Length::<Em>::new(1.);
    let line_height = one_em * style.line_height;
    let half_leading = (line_height - one_em) / 2.;
    let baseline_y = half_leading + style.font.ascender();

    let font_size = style.font_size;
    let px_per_em = font_size / one_em;
    let line_height = line_height * px_per_em;
    let baseline_y = baseline_y * px_per_em;

    let mut pdf_doc = Document::new();
    let mut line_segments = Vec::new();

    let mut previous_break_position = 0;
    let mut segments = Rewind::new(LineBreakIterator::new(text).map(
        |(position, is_hard_break)| {
            let range = previous_break_position..position;
            previous_break_position = position;
            let text_segment = text[range].trim_end_matches('\n');
            let segment = ShapedSegment::naive_shape(text_segment, style.font.clone())?;
            Ok((segment, is_hard_break))
        },
    ));

    'pages: loop {
        let mut pdf_page = pdf_doc.add_page(page_size);
        let mut y = min_y;

        'lines: loop {
            let mut total_width = Length::new(0.);
            let justify;
            loop {
                let (segment, is_hard_break) = match segments.next() {
                    Some(result) => result?,
                    // End of document
                    // FIXME: use 'return' when lifetimes are non-lexical
                    None => break 'pages,
                };

                let advance_width = segment.advance_width * px_per_em;
                let next_total_width = total_width + advance_width;
                if next_total_width > available_width && total_width > Length::new(0.) {
                    // This segment doesn’t fit on this line, and isn’t the first on the line:
                    // go to the next line.
                    segments.rewind(Ok((segment, is_hard_break)));
                    justify = style.justify;
                    break
                }
                line_segments.push(segment);
                total_width = next_total_width;
                if is_hard_break {
                    justify = false;
                    break
                }
            }

            let extra = available_width - total_width;
            let word_spacing = if justify && extra > Length::new(0.) {
                extra / (line_segments.len() - 1) as f32
            } else {
                Length::new(0.)
            };
            let baseline = y + baseline_y;
            let mut x = min_x;
            for segment in &line_segments {
                let origin = Point::from_lengths(x, baseline);
                x += segment.advance_width * px_per_em + word_spacing;
                pdf_page.show_text(&TextRun {
                    segment,
                    font_size,
                    origin,
                })?;
            }
            line_segments.clear();

            y += line_height;
            if y > max_y {
                // We’ve reached the bottom of the page
                break
            }
        }
    }
    Ok(pdf_doc)
}

struct Rewind<I>
where
    I: Iterator,
{
    inner: I,
    buffer: Option<I::Item>,
}

impl<I> Rewind<I>
where
    I: Iterator,
{
    fn new(inner: I) -> Self {
        Rewind {
            inner,
            buffer: None,
        }
    }

    fn rewind(&mut self, item: I::Item) {
        assert!(self.buffer.is_none());
        self.buffer = Some(item)
    }
}

impl<I> Iterator for Rewind<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.buffer.take().or_else(|| self.inner.next())
    }
}
