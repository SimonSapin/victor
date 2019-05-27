use super::*;
use crate::text::ShapedSegment;

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
    /// Create a fragment that does not contribute any painting operation.
    pub fn no_op() -> Self {
        Self {
            style: ComputedValues::anonymous_inheriting_from(None),
            children: vec![],
            content_rect: Rect {
                start_corner: Vec2::zero(),
                size: Vec2::zero(),
            },
            padding: Sides::zero(),
            border: Sides::zero(),
            margin: Sides::zero(),
        }
    }

    pub fn border_rect(&self) -> Rect<Length> {
        self.content_rect
            .inflate(&self.padding)
            .inflate(&self.border)
    }
}
