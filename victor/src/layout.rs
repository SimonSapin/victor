mod boxes;
pub(crate) mod fragments;

use crate::geom::flow_relative::{Rect, Vec2};
use crate::geom::Length;
use crate::style::values::{Direction, LengthOrPercentage, LengthOrPercentageOrAuto, WritingMode};
use crate::style::ComputedValues;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::sync::Arc;

impl crate::dom::Document {
    pub(crate) fn layout(
        &self,
        viewport: crate::primitives::Size<crate::primitives::CssPx>,
    ) -> Vec<fragments::Fragment> {
        let box_tree = self.box_tree();
        layout_document(&box_tree, viewport)
    }
}
fn layout_document(
    box_tree: &boxes::BoxTreeRoot,
    viewport: crate::primitives::Size<crate::primitives::CssPx>,
) -> Vec<fragments::Fragment> {
    // FIXME: use the document’s mode:
    // https://drafts.csswg.org/css-writing-modes/#principal-flow
    let initial_containing_block = ContainingBlock {
        inline_size: Length { px: viewport.width },
        block_size: Some(Length {
            px: viewport.height,
        }),
        mode: (WritingMode::HorizontalTb, Direction::Ltr),
    };

    let (fragments, _) = box_tree.layout(&initial_containing_block);
    fragments
}

struct ContainingBlock {
    inline_size: Length,
    block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

impl boxes::BlockFormattingContext {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<fragments::Fragment>, Length) {
        self.0.layout(containing_block)
    }
}

impl boxes::BlockContainer {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<fragments::Fragment>, Length) {
        match self {
            boxes::BlockContainer::BlockLevelBoxes(child_boxes) => {
                let mut child_fragments = child_boxes
                    .par_iter()
                    .map(|child| child.layout(containing_block))
                    .collect::<Vec<_>>();

                let mut block_size = Length::zero();
                for child in &mut child_fragments {
                    // FIXME: margin collapsing
                    child.content_rect.start_corner.block += block_size;
                    block_size += child.padding.block_sum()
                        + child.border.block_sum()
                        + child.margin.block_sum()
                        + child.content_rect.size.block;
                }

                (child_fragments, block_size)
            }
            boxes::BlockContainer::InlineFormattingContext(_children) => {
                eprintln!("Unimplemented: inline formatting context");
                (Vec::new(), Length::zero())
            }
        }
    }
}

impl boxes::BlockLevelBox {
    fn layout(&self, containing_block: &ContainingBlock) -> fragments::Fragment {
        match self {
            boxes::BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                same_formatting_context_block(style, contents, containing_block)
            }
        }
    }
}

fn same_formatting_context_block(
    style: &Arc<ComputedValues>,
    contents: &boxes::BlockContainer,
    containing_block: &ContainingBlock,
) -> fragments::Fragment {
    let cbis = containing_block.inline_size;
    let zero = Length::zero();
    let padding = style.padding().map(|v| v.percentage_relative_to(cbis));
    let border = style.border_width().map(|v| v.percentage_relative_to(cbis));
    let pb = &padding + &border;
    let box_size = style.box_size();
    let mut computed_margin = style.margin();
    let inline_size;
    let margin;
    if let Some(is) = box_size.inline.non_auto() {
        let is = is.percentage_relative_to(cbis);
        let inline_margins = cbis - is - pb.inline_sum();
        inline_size = Some(is);
        use LengthOrPercentageOrAuto as LPA;
        match (
            &mut computed_margin.inline_start,
            &mut computed_margin.inline_end,
        ) {
            (s @ &mut LPA::Auto, e @ &mut LPA::Auto) => {
                *s = LPA::Length(inline_margins / 2.);
                *e = LPA::Length(inline_margins / 2.);
            }
            (s @ &mut LPA::Auto, _) => {
                *s = LPA::Length(inline_margins);
            }
            (_, e @ &mut LPA::Auto) => {
                *e = LPA::Length(inline_margins);
            }
            (_, e @ _) => {
                // Either the inline-end margin is auto,
                // or we’re over-constrained and we do as if it were.
                *e = LPA::Length(inline_margins);
            }
        }
        margin = computed_margin
            .map_inline_and_block_axes(|v| v.auto_is(|| unreachable!()), |v| v.auto_is(|| zero));
    } else {
        inline_size = None; // auto
        margin = computed_margin.map(|v| v.auto_is(|| zero));
    }
    let margin = margin.map(|v| v.percentage_relative_to(cbis));
    let pbm = &pb + &margin;
    let inline_size = inline_size.unwrap_or_else(|| cbis - pbm.inline_sum());
    let block_size = box_size.block.non_auto().and_then(|b| match b {
        LengthOrPercentage::Length(l) => Some(l),
        LengthOrPercentage::Percentage(p) => containing_block.block_size.map(|cbbs| cbbs * p),
    });
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
    let (children, content_block_size) = contents.layout(&containing_block_for_children);
    let block_size = block_size.unwrap_or(content_block_size);
    let content_rect = Rect {
        start_corner: pbm.start_corner(),
        size: Vec2 {
            block: block_size,
            inline: inline_size,
        },
    };
    fragments::Fragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
    }
}
