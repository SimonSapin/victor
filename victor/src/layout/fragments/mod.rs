use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::ComputedValues;
use crate::text::ShapedSegment;
use std::sync::Arc;

pub(crate) enum Fragment {
    Box(BoxFragment),
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
    pub parent_style: Arc<ComputedValues>,
    pub content_rect: Rect<Length>,
    pub text: ShapedSegment,
}

impl BoxFragment {
    pub fn zero_sized(style: Arc<ComputedValues>) -> Self {
        let zero_vec = Vec2 {
            inline: Length::zero(),
            block: Length::zero(),
        };
        let zero_sides = Sides {
            inline_start: Length::zero(),
            inline_end: Length::zero(),
            block_start: Length::zero(),
            block_end: Length::zero(),
        };
        Self {
            style,
            children: vec![],
            content_rect: Rect {
                start_corner: zero_vec.clone(),
                size: zero_vec,
            },
            padding: zero_sides.clone(),
            border: zero_sides.clone(),
            margin: zero_sides,
        }
    }

    pub fn border_rect(&self) -> Rect<Length> {
        self.content_rect
            .inflate(&self.padding)
            .inflate(&self.border)
    }
}
