mod boxes;
pub(crate) mod fragments;

use self::boxes::*;
use self::fragments::*;
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
    ) -> Vec<Fragment> {
        let box_tree = self.box_tree();
        layout_document(&box_tree, viewport)
    }
}
fn layout_document(
    box_tree: &BoxTreeRoot,
    viewport: crate::primitives::Size<crate::primitives::CssPx>,
) -> Vec<Fragment> {
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

impl BlockFormattingContext {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<Fragment>, Length) {
        self.0.layout(containing_block)
    }
}

impl BlockContainer {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<Fragment>, Length) {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => {
                let mut child_fragments = child_boxes
                    .par_iter()
                    .map(|child| child.layout(containing_block))
                    .collect::<Vec<_>>();

                let mut block_size = Length::zero();
                for child in &mut child_fragments {
                    let child = match child {
                        Fragment::Box(b) => b,
                        _ => unreachable!(),
                    };
                    // FIXME: margin collapsing
                    child.content_rect.start_corner.block += block_size;
                    block_size += child.padding.block_sum()
                        + child.border.block_sum()
                        + child.margin.block_sum()
                        + child.content_rect.size.block;
                }

                (child_fragments, block_size)
            }
            BlockContainer::InlineFormattingContext(ifc) => ifc.layout(containing_block),
        }
    }
}

impl BlockLevelBox {
    fn layout(&self, containing_block: &ContainingBlock) -> Fragment {
        match self {
            BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                same_formatting_context_block(style, contents, containing_block)
            }
        }
    }
}

fn same_formatting_context_block(
    style: &Arc<ComputedValues>,
    contents: &BlockContainer,
    containing_block: &ContainingBlock,
) -> Fragment {
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
    Fragment::Box(BoxFragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
    })
}

impl InlineFormattingContext {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<Fragment>, Length) {
        self.layout_inline_level_boxes(
            containing_block,
            &self.inline_level_boxes,
            &mut Length::zero(),
        )
    }

    fn layout_inline_level_boxes(
        &self,
        containing_block: &ContainingBlock,
        children: &[InlineLevelBox],
        inline_position: &mut Length,
    ) -> (Vec<Fragment>, Length) {
        let mut block_size = Length::zero();
        let fragments = children
            .iter()
            .map(|child| {
                let (fragment, child_block_size) =
                    child.layout(containing_block, self, inline_position);
                block_size = block_size.max(child_block_size);
                fragment
            })
            .collect();
        (fragments, block_size)
    }
}

impl InlineLevelBox {
    fn layout(
        &self,
        containing_block: &ContainingBlock,
        formatting_context: &InlineFormattingContext,
        inline_position: &mut Length,
    ) -> (Fragment, Length) {
        match self {
            InlineLevelBox::InlineBox(inline) => {
                inline.layout(containing_block, formatting_context, inline_position)
            }
            InlineLevelBox::TextRun(id) => {
                formatting_context.text_runs[id.0].layout(inline_position)
            }
        }
    }
}

impl InlineBox {
    fn layout(
        &self,
        containing_block: &ContainingBlock,
        formatting_context: &InlineFormattingContext,
        inline_position: &mut Length,
    ) -> (Fragment, Length) {
        let style = self.style.clone();
        let cbis = containing_block.inline_size;
        let mut padding = style.padding().map(|v| v.percentage_relative_to(cbis));
        let mut border = style.border_width().map(|v| v.percentage_relative_to(cbis));
        let mut margin = style
            .margin()
            .map(|v| v.auto_is(Length::zero).percentage_relative_to(cbis));
        if self.first_fragment {
            *inline_position += padding.inline_start + border.inline_start + margin.inline_start;
        } else {
            padding.inline_start = Length::zero();
            border.inline_start = Length::zero();
            margin.inline_start = Length::zero();
        }
        let content_inline_start = *inline_position;
        let (children, mut block_size) = formatting_context.layout_inline_level_boxes(
            containing_block,
            &self.children,
            inline_position,
        );
        let content_rect = Rect {
            start_corner: Vec2 {
                block: padding.block_start + border.block_start + margin.block_start,
                inline: content_inline_start,
            },
            size: Vec2 {
                block: block_size,
                inline: *inline_position - content_inline_start,
            },
        };
        block_size += padding.block_sum() + border.block_sum() + margin.block_sum();
        if self.last_fragment {
            *inline_position += padding.inline_end + border.inline_end + margin.inline_end;
        } else {
            padding.inline_end = Length::zero();
            border.inline_end = Length::zero();
            margin.inline_end = Length::zero();
        }
        let fragment = Fragment::Box(BoxFragment {
            style,
            children,
            content_rect,
            padding,
            border,
            margin,
        });
        (fragment, block_size)
    }
}

impl TextRun {
    fn layout(&self, inline_position: &mut Length) -> (Fragment, Length) {
        let parent_style = self.parent_style.clone();
        let inline_size = parent_style.font.font_size * self.segment.advance_width;
        // https://www.w3.org/TR/CSS2/visudet.html#propdef-line-height
        // 'normal':
        // “set the used value to a "reasonable" value based on the font of the element.”
        let line_height = parent_style.font.font_size.0 * 1.2;
        let content_rect = Rect {
            start_corner: Vec2 {
                block: Length::zero(),
                inline: *inline_position,
            },
            size: Vec2 {
                block: line_height,
                inline: inline_size,
            },
        };
        *inline_position += inline_size;
        let fragment = Fragment::Text(TextFragment {
            parent_style,
            content_rect,
            // FIXME: keep Arc<ShapedSegment> instead of ShapedSegment,
            // to make this clone cheaper?
            text: self.segment.clone(),
        });
        (fragment, line_height)
    }
}
