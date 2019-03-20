use crate::geom::flow_relative::{Rect, Sides};
use crate::geom::Length;
use crate::style::ComputedValues;
use std::sync::Arc;

pub(crate) struct Fragment {
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

impl Fragment {
    pub fn border_rect(&self) -> Rect<Length> {
        self.content_rect
            .inflate(&self.padding)
            .inflate(&self.border)
    }
}
