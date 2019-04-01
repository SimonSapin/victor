use crate::geom::flow_relative::{Rect, Sides};
use crate::geom::Length;
use crate::style::ComputedValues;
use crate::text::ShapedSegment;
use std::sync::Arc;

pub(crate) enum Fragment {
    Box(BoxFragment),
    #[allow(unused)]
    Text(TextFragment),
}

pub(crate) struct BoxFragment {
    pub style: Arc<ComputedValues>,
    pub children: Vec<Fragment>,

    /// From the containing block’s start corner…?
    /// This might be broken when the containing block is in a different writing mode:
    /// https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
    pub content_rect: Rect<Length>,

    pub padding: Sides<Length>,
    pub border: Sides<Length>,
    pub margin: Sides<Length>,
}

pub(crate) struct TextFragment {
    pub style: Arc<ComputedValues>,
    pub content_rect: Rect<Length>,
    pub text: ShapedSegment,
}

impl BoxFragment {
    pub fn border_rect(&self) -> Rect<Length> {
        self.content_rect
            .inflate(&self.padding)
            .inflate(&self.border)
    }
}
