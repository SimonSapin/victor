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
        contents: IndependentFormattingContext,
    },
}

impl BlockFormattingContext {
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        let mut float_context;
        let float_context = if self.contains_floats {
            float_context = FloatContext::new();
            Some(&mut float_context)
        } else {
            None
        };
        self.contents
            .layout(containing_block, float_context, tree_rank)
    }
}

impl BlockContainer {
    pub fn layout(
        &self,
        containing_block: &ContainingBlock,
        float_context: Option<&mut FloatContext>,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => {
                layout_block_level_children(containing_block, float_context, tree_rank, child_boxes)
            }
            BlockContainer::InlineFormattingContext(ifc) => ifc.layout(containing_block, tree_rank),
        }
    }
}

fn layout_block_level_children<'a>(
    containing_block: &ContainingBlock,
    float_context: Option<&mut FloatContext>,
    tree_rank: usize,
    child_boxes: &'a [BlockLevelBox],
) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment<'a>>, Length) {
    let mut absolutely_positioned_fragments = vec![];
    let mut current_block_direction_position = Length::zero();
    let mut child_fragments: Vec<Fragment>;
    if let Some(float_context) = float_context {
        // Because floats are involved, we do layout for this block formatting context
        // in tree order without parallelism. This enables mutable access
        // to a `FloatContext` that tracks every float encountered so far (again in tree order).
        child_fragments = child_boxes
            .iter()
            .enumerate()
            .map(|(tree_rank, box_)| {
                let mut fragment = box_.layout(
                    containing_block,
                    Some(float_context),
                    tree_rank,
                    &mut absolutely_positioned_fragments,
                );
                place_block_level_fragment(&mut fragment, &mut current_block_direction_position);
                fragment
            })
            .collect()
    } else {
        child_fragments = child_boxes
            .par_iter()
            .enumerate()
            .mapfold_reduce_into(
                &mut absolutely_positioned_fragments,
                |abspos_fragments, (tree_rank, box_)| {
                    box_.layout(
                        containing_block,
                        /* float_context = */ None,
                        tree_rank,
                        abspos_fragments,
                    )
                },
                |left_abspos_fragments, mut right_abspos_fragments| {
                    left_abspos_fragments.append(&mut right_abspos_fragments);
                },
            )
            .collect();
        for fragment in &mut child_fragments {
            place_block_level_fragment(fragment, &mut current_block_direction_position)
        }
    }
    let content_block_size = current_block_direction_position;
    let block_size = containing_block.block_size.auto_is(|| content_block_size);

    adjust_static_positions(
        &mut absolutely_positioned_fragments,
        &mut child_fragments,
        tree_rank,
    );

    (child_fragments, absolutely_positioned_fragments, block_size)
}

fn place_block_level_fragment(
    fragment: &mut Fragment,
    current_block_direction_position: &mut Length,
) {
    let (bpm, rect) = match fragment {
        Fragment::Box(fragment) => (
            fragment.padding.block_sum()
                + fragment.border.block_sum()
                + fragment.margin.block_sum(),
            &mut fragment.content_rect,
        ),
        Fragment::Anonymous(fragment) => (Length::zero(), &mut fragment.rect),
        _ => unreachable!(),
    };
    // FIXME: margin collapsing
    rect.start_corner.block += *current_block_direction_position;
    *current_block_direction_position += bpm + rect.size.block;
}

impl BlockLevelBox {
    fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        float_context: Option<&mut FloatContext>,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    ) -> Fragment {
        match self {
            BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                Fragment::Box(layout_in_flow_non_replaced_block_level(
                    containing_block,
                    absolutely_positioned_fragments,
                    style,
                    |containing_block| contents.layout(containing_block, float_context, tree_rank),
                ))
            }
            BlockLevelBox::Independent { style, contents } => match contents.as_replaced() {
                Ok(replaced) => {
                    // FIXME
                    match *replaced {}
                }
                Err(contents) => Fragment::Box(layout_in_flow_non_replaced_block_level(
                    containing_block,
                    absolutely_positioned_fragments,
                    style,
                    |containing_block| contents.layout(containing_block, tree_rank),
                )),
            },
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

/// https://drafts.csswg.org/css2/visudet.html#blockwidth
/// https://drafts.csswg.org/css2/visudet.html#normal-block
fn layout_in_flow_non_replaced_block_level<'a>(
    containing_block: &ContainingBlock,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    style: &Arc<ComputedValues>,
    layout_contents: impl FnOnce(
        &ContainingBlock,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment<'a>>, Length),
) -> BoxFragment {
    let cbis = containing_block.inline_size;
    let padding = style.padding().percentages_relative_to(cbis);
    let border = style.border_width().percentages_relative_to(cbis);
    let mut computed_margin = style.margin().percentages_relative_to(cbis);
    let pb = &padding + &border;
    let box_size = style.box_size();
    let inline_size = box_size.inline.percentage_relative_to(cbis);
    if let LengthOrAuto::Length(is) = inline_size {
        let inline_margins = cbis - is - pb.inline_sum();
        use LengthOrAuto::*;
        match (
            &mut computed_margin.inline_start,
            &mut computed_margin.inline_end,
        ) {
            (s @ &mut Auto, e @ &mut Auto) => {
                *s = Length(inline_margins / 2.);
                *e = Length(inline_margins / 2.);
            }
            (s @ &mut Auto, _) => {
                *s = Length(inline_margins);
            }
            (_, e @ &mut Auto) => {
                *e = Length(inline_margins);
            }
            (_, e @ _) => {
                // Either the inline-end margin is auto,
                // or we’re over-constrained and we do as if it were.
                *e = Length(inline_margins);
            }
        }
    }
    let margin = computed_margin.auto_is(Length::zero);
    let pbm = &pb + &margin;
    let inline_size = inline_size.auto_is(|| cbis - pbm.inline_sum());
    let block_size = match box_size.block {
        LengthOrPercentageOrAuto::Length(l) => LengthOrAuto::Length(l),
        LengthOrPercentageOrAuto::Percentage(p) => containing_block.block_size.map(|cbbs| cbbs * p),
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
    let (mut children, nested_abspos, content_block_size) =
        layout_contents(&containing_block_for_children);
    let relative_adjustement = relative_adjustement(style, inline_size, block_size);
    let block_size = block_size.auto_is(|| content_block_size);
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
    BoxFragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
    }
}
