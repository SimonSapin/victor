use super::*;
use crate::geom::flow_relative::FloatAnchor;
use crate::geom::physical;

#[derive(Debug)]
pub(in crate::layout) struct FloatBox {
    pub style: Arc<ComputedValues>,
    pub contents: IndependentFormattingContext,
    pub anchor: physical::FloatAnchor,
}

pub(in crate::layout) struct FloatContext<'list> {
    start_corner_in_bfc: Vec2<Length>,
    inline_size: Length,
    list: &'list mut FloatList,
}

pub(super) struct FloatList {
    spaces: Vec<Space>,
    next_float_block_start: Length,
}

struct Space {
    inline_edge: Length,
    block_start: Length,
    block_size: Length,
    anchor: FloatAnchor,
}

impl ContainsFloats {
    pub(super) fn into_list(self) -> Option<FloatList> {
        match self {
            ContainsFloats::Yes => Some(FloatList {
                spaces: vec![],
                next_float_block_start: Length::zero(),
            }),
            ContainsFloats::No => None,
        }
    }
}

impl<'list> FloatContext<'list> {
    pub(super) fn root(inline_size: Length, list: &'list mut FloatList) -> Self {
        Self {
            start_corner_in_bfc: Vec2::zero(),
            inline_size,
            list,
        }
    }

    pub(super) fn child(
        &mut self,
        inline_size: Length,
        child_start_corner_from_margin_rect: &Vec2<Length>,
    ) -> FloatContext {
        FloatContext {
            start_corner_in_bfc: &self.start_corner_in_bfc + child_start_corner_from_margin_rect,
            inline_size,
            list: self.list,
        }
    }

    pub(super) fn advance_block_start(&mut self, increment: Length) {
        self.start_corner_in_bfc.block += increment;
    }
}

impl Space {
    fn block_end(&self) -> Length {
        self.block_start + self.block_size
    }
}

impl FloatBox {
    fn anchor(&self, containing_block: &ContainingBlock) -> FloatAnchor {
        self.anchor.to_flow_relative(containing_block.mode)
    }

    fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
        tree_rank: usize,
    ) -> BoxFragment {
        let cbis = containing_block.inline_size;
        let style = &self.style;

        let padding = style.padding().percentages_relative_to(cbis);
        let border = style.border_width().percentages_relative_to(cbis);
        let margin = style
            .margin()
            .percentages_relative_to(cbis)
            .auto_is(Length::zero);
        let pb = &padding + &border;
        let pbm = &pb + &margin;

        let box_size = style.box_size();

        let inline_size = box_size.inline.percentage_relative_to(cbis).auto_is(|| {
            let available_size = containing_block.inline_size;

            // FIXME(nox): shrink-to-fit.
            available_size
        });
        let block_size = match box_size.block {
            LengthOrPercentageOrAuto::Length(l) => LengthOrAuto::Length(l),
            LengthOrPercentageOrAuto::Percentage(p) => {
                containing_block.block_size.map(|cbbs| cbbs * p)
            }
            LengthOrPercentageOrAuto::Auto => LengthOrAuto::Auto,
        };

        let containing_block_for_children = ContainingBlock {
            inline_size,
            block_size,
            mode: style.writing_mode(),
        };
        // https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
        assert_eq!(
            containing_block.mode, containing_block_for_children.mode,
            "Mixed writing modes are not supported yet"
        );

        let start_corner = pbm.start_corner();
        let (mut children, nested_abspos, content_block_size) = self
            .contents
            .layout(&containing_block_for_children, tree_rank);
        let relative_adjustement = relative_adjustement(style, inline_size, block_size);
        let block_size = block_size.auto_is(|| content_block_size);
        let content_rect = Rect {
            start_corner: &start_corner + &relative_adjustement,
            size: Vec2 {
                block: block_size,
                inline: inline_size,
            },
        };
        if style.box_.position.is_relatively_positioned() {
            AbsolutelyPositionedFragment::in_positioned_containing_block(
                &nested_abspos,
                &mut children,
                &content_rect.size,
                &padding,
                containing_block_for_children.mode,
            )
        } else {
            absolutely_positioned_fragments.extend(nested_abspos);
        };
        BoxFragment {
            style: style.clone(),
            children,
            content_rect,
            padding,
            border,
            margin,
        }
    }
}
