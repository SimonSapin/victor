use super::*;
use crate::text::ShapedSegment;

pub(crate) enum Fragment {
    Box(BoxFragment),
    Anonymous(AnonymousFragment),
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

/// Can contain child fragments with relative coordinates, but does not contribute to painting itself.
pub(crate) struct AnonymousFragment {
    pub rect: Rect<Length>,
    pub children: Vec<Fragment>,
    pub mode: (WritingMode, Direction),
}

pub(crate) struct TextFragment {
    pub parent_style: Arc<ComputedValues>,
    pub content_rect: Rect<Length>,
    pub text: ShapedSegment,
}

impl AnonymousFragment {
    pub fn no_op(mode: (WritingMode, Direction)) -> Self {
        Self {
            children: vec![],
            rect: Rect::zero(),
            mode,
        }
    }
}

impl BoxFragment {
    pub fn border_rect(&self) -> Rect<Length> {
        self.content_rect
            .inflate(&self.padding)
            .inflate(&self.border)
    }
}
