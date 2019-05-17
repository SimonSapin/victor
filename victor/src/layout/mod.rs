mod abspos;
mod boxes;
pub(crate) mod fragments;
mod inline;

use self::abspos::*;
use self::boxes::*;
use self::fragments::*;
use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::values::{Direction, LengthOrPercentage, LengthOrPercentageOrAuto, WritingMode};
use crate::style::ComputedValues;
use std::sync::Arc;

pub(crate) use self::boxes::ReplacedContent;

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
    let inline_size = Length { px: viewport.width };

    // FIXME: use the document’s mode:
    // https://drafts.csswg.org/css-writing-modes/#principal-flow
    let initial_containing_block = ContainingBlock {
        inline_size,
        block_size: Some(Length {
            px: viewport.height,
        }),
        mode: (WritingMode::HorizontalTb, Direction::Ltr),
    };

    let zero = Length::zero();
    let initial_containing_block_padding = Sides {
        inline_start: zero,
        inline_end: zero,
        block_start: zero,
        block_end: zero,
    };

    let (fragments, _) = box_tree.contents.layout_into_absolute_containing_block(
        &initial_containing_block,
        &initial_containing_block_padding,
    );
    fragments
}

struct ContainingBlock {
    inline_size: Length,
    block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

impl BlockContainer {
    fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => {
                let (mut child_fragments, mut absolutely_positioned_fragments) =
                    layout_block_level_children(containing_block, child_boxes);

                let mut content_block_size = Length::zero();
                for child in &mut child_fragments {
                    let child = match child {
                        Fragment::Box(b) => b,
                        _ => unreachable!(),
                    };
                    // FIXME: margin collapsing
                    child.content_rect.start_corner.block += content_block_size;
                    content_block_size += child.padding.block_sum()
                        + child.border.block_sum()
                        + child.margin.block_sum()
                        + child.content_rect.size.block;
                }

                for abspos_fragment in &mut absolutely_positioned_fragments {
                    let child_fragment = match &child_fragments[abspos_fragment.tree_rank] {
                        Fragment::Box(b) => b,
                        _ => unreachable!(),
                    };

                    abspos_fragment.tree_rank = tree_rank;

                    if let AbsoluteBoxOffsets::StaticStart { start } =
                        &mut abspos_fragment.inline_start
                    {
                        *start += child_fragment.content_rect.start_corner.inline;
                    }

                    if let AbsoluteBoxOffsets::StaticStart { start } =
                        &mut abspos_fragment.block_start
                    {
                        *start += child_fragment.content_rect.start_corner.block;
                    }
                }

                let block_size = containing_block.block_size.unwrap_or(content_block_size);

                (child_fragments, absolutely_positioned_fragments, block_size)
            }
            BlockContainer::InlineFormattingContext(ifc) => ifc.layout(containing_block, tree_rank),
        }
    }
}

fn layout_block_level_children<'a>(
    containing_block: &ContainingBlock,
    child_boxes: &'a [BlockLevelBox],
) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment<'a>>) {
    use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
    use rayon_croissant::ParallelIteratorExt;

    let mut absolutely_positioned_fragments = vec![];
    let child_fragments = child_boxes
        .par_iter()
        .enumerate()
        .mapfold_reduce_into(
            &mut absolutely_positioned_fragments,
            |abspos_fragments, (tree_rank, child)| {
                child.layout(containing_block, tree_rank, abspos_fragments)
            },
            |left_abspos_fragments, mut right_abspos_fragments| {
                left_abspos_fragments.append(&mut right_abspos_fragments);
            },
        )
        .collect::<Vec<_>>();

    (child_fragments, absolutely_positioned_fragments)
}

impl BlockLevelBox {
    fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    ) -> Fragment {
        match self {
            BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                same_formatting_context_block(
                    containing_block,
                    tree_rank,
                    absolutely_positioned_fragments,
                    style,
                    contents,
                )
            }
            BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(box_) => {
                absolutely_positioned_fragments.push(box_.layout(Vec2::zero(), tree_rank));
                Fragment::Box(BoxFragment::no_op())
            }
            BlockLevelBox::OutOfFlowFloatBox(_box_) => {
                // TODO
                Fragment::Box(BoxFragment::no_op())
            }
        }
    }
}

fn same_formatting_context_block<'a>(
    containing_block: &ContainingBlock,
    tree_rank: usize,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    style: &Arc<ComputedValues>,
    contents: &'a boxes::BlockContainer,
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
    let (children, content_block_size) = if !style.box_.position.is_relatively_positioned() {
        let (children, abspos_children, content_block_size) =
            contents.layout(&containing_block_for_children, tree_rank);
        absolutely_positioned_fragments.extend(abspos_children);
        (children, content_block_size)
    } else {
        contents.layout_into_absolute_containing_block(&containing_block_for_children, &padding)
    };
    let relative_adjustement = relative_adjustement(style, inline_size, block_size);
    let block_size = block_size.unwrap_or(content_block_size);
    let content_rect = Rect {
        start_corner: &pbm.start_corner() + &relative_adjustement,
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

/// https://drafts.csswg.org/css2/visuren.html#relative-positioning
fn relative_adjustement(
    style: &ComputedValues,
    inline_size: Length,
    block_size: Option<Length>,
) -> Vec2<Length> {
    if !style.box_.position.is_relatively_positioned() {
        return Vec2::zero();
    }
    fn adjust(start: Option<Length>, end: Option<Length>) -> Length {
        match (start, end) {
            (None, None) => Length::zero(),
            (Some(start), _) => start,
            (None, Some(end)) => -end,
        }
    }
    let block_size = block_size.unwrap_or(Length::zero());
    let box_offsets = style.box_offsets().map_inline_and_block_axes(
        |v| v.non_auto().map(|v| v.percentage_relative_to(inline_size)),
        |v| v.non_auto().map(|v| v.percentage_relative_to(block_size)),
    );
    Vec2 {
        inline: adjust(box_offsets.inline_start, box_offsets.inline_end),
        block: adjust(box_offsets.block_start, box_offsets.block_end),
    }
}

trait Take {
    fn take(&mut self) -> Self;
}

impl<T> Take for T
where
    T: Default,
{
    fn take(&mut self) -> Self {
        std::mem::replace(self, Default::default())
    }
}
