//! Flow layout, also known as block-and-inline layout.

use super::*;
use rayon::prelude::*;
use rayon_croissant::ParallelIteratorExt;

mod construct;
mod float;
mod inline;
mod root;

pub(super) use construct::*;
pub(super) use float::*;
use inline::*;

#[derive(Debug)]
pub(super) struct BlockFormattingContext {
    pub contents: BlockContainer,
    pub contains_floats: bool,
}

#[derive(Debug)]
pub(super) enum BlockContainer {
    BlockLevelBoxes(Vec<BlockLevelBox>),
    InlineFormattingContext(InlineFormattingContext),
}

#[derive(Debug)]
pub(super) enum BlockLevelBox {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: BlockContainer,
    },
    OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox),
    OutOfFlowFloatBox(FloatBox),
    Independent {
        style: Arc<ComputedValues>,
        // FIXME: this should be IndependentFormattingContext:
        contents: ReplacedContent,
    },
}

impl BlockFormattingContext {
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        let dummy_tree_rank = 0;
        self.contents.layout(containing_block, dummy_tree_rank)
    }
}

impl BlockContainer {
    pub(super) fn layout(
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
                    let (bpm, rect) = match child {
                        Fragment::Box(child) => (
                            child.padding.block_sum()
                                + child.border.block_sum()
                                + child.margin.block_sum(),
                            &mut child.content_rect,
                        ),
                        Fragment::Anonymous(child) => (Length::zero(), &mut child.rect),
                        _ => unreachable!(),
                    };
                    // FIXME: margin collapsing
                    rect.start_corner.block += content_block_size;
                    content_block_size += bpm + rect.size.block;
                }

                for abspos_fragment in &mut absolutely_positioned_fragments {
                    let child_fragment_rect = match &child_fragments[abspos_fragment.tree_rank] {
                        Fragment::Box(b) => &b.content_rect,
                        Fragment::Anonymous(a) => &a.rect,
                        _ => unreachable!(),
                    };

                    abspos_fragment.tree_rank = tree_rank;

                    if let AbsoluteBoxOffsets::StaticStart { start } =
                        &mut abspos_fragment.inline_start
                    {
                        *start += child_fragment_rect.start_corner.inline;
                    }

                    if let AbsoluteBoxOffsets::StaticStart { start } =
                        &mut abspos_fragment.block_start
                    {
                        *start += child_fragment_rect.start_corner.block;
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
            BlockLevelBox::Independent { style: _, contents } => {
                // FIXME
                match *contents {}
            }
            BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(box_) => {
                absolutely_positioned_fragments.push(box_.layout(Vec2::zero(), tree_rank));
                Fragment::Anonymous(AnonymousFragment::no_op(containing_block.mode))
            }
            BlockLevelBox::OutOfFlowFloatBox(_box_) => {
                // TODO
                Fragment::Anonymous(AnonymousFragment::no_op(containing_block.mode))
            }
        }
    }
}

fn same_formatting_context_block<'a>(
    containing_block: &ContainingBlock,
    tree_rank: usize,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    style: &Arc<ComputedValues>,
    contents: &'a BlockContainer,
) -> Fragment {
    let cbis = containing_block.inline_size;
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
        margin = computed_margin.map_inline_and_block_axes(
            |v| v.auto_is(|| unreachable!()),
            |v| v.auto_is(Length::zero),
        );
    } else {
        inline_size = None; // auto
        margin = computed_margin.map(|v| v.auto_is(Length::zero));
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
    let (mut children, nested_abspos, content_block_size) =
        contents.layout(&containing_block_for_children, tree_rank);
    let relative_adjustement = relative_adjustement(style, inline_size, block_size);
    let block_size = block_size.unwrap_or(content_block_size);
    let content_rect = Rect {
        start_corner: &pbm.start_corner() + &relative_adjustement,
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
    Fragment::Box(BoxFragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
    })
}
