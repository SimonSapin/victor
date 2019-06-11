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

pub(super) struct FlowChildren<'a> {
    pub fragments: Vec<Fragment>,
    pub absolutely_positioned_fragments: Vec<AbsolutelyPositionedFragment<'a>>,
    pub block_size: Length,
    pub collapsing_context: CollapsingContext,
}

#[derive(Clone, Copy)]
struct CollapsibleWithParentStartMargin(bool);

impl BlockFormattingContext {
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> FlowChildren {
        let mut float_context;
        let float_context = if self.contains_floats {
            float_context = FloatContext::new();
            Some(&mut float_context)
        } else {
            None
        };
        let mut flow_children = self.contents.layout(
            containing_block,
            float_context,
            tree_rank,
            CollapsibleWithParentStartMargin(false),
        );
        flow_children.block_size += flow_children.collapsing_context.end.solve();
        flow_children.collapsing_context.end = CollapsedMargin::zero();
        flow_children
    }
}

impl BlockContainer {
    fn layout(
        &self,
        containing_block: &ContainingBlock,
        float_context: Option<&mut FloatContext>,
        tree_rank: usize,
        collapsible_with_parent_start_margin: CollapsibleWithParentStartMargin,
    ) -> FlowChildren {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => layout_block_level_children(
                containing_block,
                float_context,
                tree_rank,
                collapsible_with_parent_start_margin,
                child_boxes,
            ),
            BlockContainer::InlineFormattingContext(ifc) => ifc.layout(containing_block, tree_rank),
        }
    }
}

fn layout_block_level_children<'a>(
    containing_block: &ContainingBlock,
    float_context: Option<&mut FloatContext>,
    tree_rank: usize,
    collapsible_with_parent_start_margin: CollapsibleWithParentStartMargin,
    child_boxes: &'a [BlockLevelBox],
) -> FlowChildren<'a> {
    fn place_block_level_fragment(
        fragment: &mut Fragment,
        current_block_direction_position: &mut Length,
        ongoing_collapsed_margin: &mut OngoingCollapsedMargin,
    ) {
        match fragment {
            Fragment::Box(fragment) => {
                let mut fragment_block_size = fragment.padding.block_sum()
                    + fragment.border.block_sum()
                    + fragment.content_rect.size.block;
                if let Some(collapsing_context) = &fragment.collapsing_context {
                    *current_block_direction_position +=
                        ongoing_collapsed_margin.adjoin(collapsing_context);
                } else {
                    fragment_block_size += fragment.margin.block_sum();
                    ongoing_collapsed_margin.reset_end();
                }
                fragment.content_rect.start_corner.block += *current_block_direction_position;
                *current_block_direction_position += fragment_block_size;
            }
            Fragment::Anonymous(fragment) => {
                // FIXME(nox): Margin collapsing for hypothetical boxes of
                // abspos elements is probably wrong.
                assert!(fragment.children.is_empty());
                assert_eq!(fragment.rect.size.block, Length::zero());
                fragment.rect.start_corner.block += *current_block_direction_position;
            }
            _ => unreachable!(),
        }
    }

    struct OngoingCollapsedMargin {
        collapsed_with_parent_start_margin: bool,
        context: CollapsingContext,
    }

    impl OngoingCollapsedMargin {
        fn adjoin(&mut self, other: &CollapsingContext) -> Length {
            if self.collapsed_with_parent_start_margin.take() {
                assert_eq!(self.context.end.solve(), Length::zero());
                self.context.start.adjoin_assign(&other.start);
                self.context.end = other.end;
                return Length::zero();
            }
            let margin = self.context.end.adjoin(&other.start).solve();
            self.context.end = other.end;
            margin
        }

        fn reset_end(&mut self) {
            self.context.end = CollapsedMargin::zero();
        }
    }

    let mut absolutely_positioned_fragments = vec![];
    let mut current_block_direction_position = Length::zero();
    let mut ongoing_collapsed_margin = OngoingCollapsedMargin {
        collapsed_with_parent_start_margin: collapsible_with_parent_start_margin.0,
        context: CollapsingContext::zero(),
    };
    let mut fragments: Vec<_>;
    if let Some(float_context) = float_context {
        // Because floats are involved, we do layout for this block formatting context
        // in tree order without parallelism. This enables mutable access
        // to a `FloatContext` that tracks every float encountered so far (again in tree order).
        fragments = child_boxes
            .iter()
            .enumerate()
            .map(|(tree_rank, box_)| {
                let mut fragment = box_.layout(
                    containing_block,
                    Some(float_context),
                    tree_rank,
                    &mut absolutely_positioned_fragments,
                );
                place_block_level_fragment(
                    &mut fragment,
                    &mut current_block_direction_position,
                    &mut ongoing_collapsed_margin,
                );
                fragment
            })
            .collect()
    } else {
        fragments = child_boxes
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
        for fragment in &mut fragments {
            place_block_level_fragment(
                fragment,
                &mut current_block_direction_position,
                &mut ongoing_collapsed_margin,
            )
        }
    }

    adjust_static_positions(
        &mut absolutely_positioned_fragments,
        &mut fragments,
        tree_rank,
    );

    FlowChildren {
        fragments,
        absolutely_positioned_fragments,
        block_size: current_block_direction_position,
        collapsing_context: ongoing_collapsed_margin.context,
    }
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
                    BlockLevelKind::SameFormattingContextBlock,
                    |containing_block, collapsible_with_parent_start_margin| {
                        contents.layout(
                            containing_block,
                            float_context,
                            tree_rank,
                            collapsible_with_parent_start_margin,
                        )
                    },
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
                    BlockLevelKind::EstablishesAnIndependentFormattingContext,
                    |containing_block, _| contents.layout(containing_block, tree_rank),
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

enum BlockLevelKind {
    SameFormattingContextBlock,
    EstablishesAnIndependentFormattingContext,
}

/// https://drafts.csswg.org/css2/visudet.html#blockwidth
/// https://drafts.csswg.org/css2/visudet.html#normal-block
fn layout_in_flow_non_replaced_block_level<'a>(
    containing_block: &ContainingBlock,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    style: &Arc<ComputedValues>,
    block_level_kind: BlockLevelKind,
    layout_contents: impl FnOnce(&ContainingBlock, CollapsibleWithParentStartMargin) -> FlowChildren<'a>,
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
    let (mut collapsing_context, initial_margin_block_start) = match block_level_kind {
        BlockLevelKind::SameFormattingContextBlock => (
            Some(CollapsingContext::from_margin(&margin)),
            Length::zero(),
        ),
        BlockLevelKind::EstablishesAnIndependentFormattingContext => (None, margin.block_start),
    };
    let inline_size = inline_size.auto_is(|| cbis - pb.inline_sum() - margin.inline_sum());
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
    let collapsible_with_parent_start_margin = CollapsibleWithParentStartMargin(
        matches!(block_level_kind, BlockLevelKind::SameFormattingContextBlock)
            && pb.block_start == Length::zero(),
    );
    let mut flow_children = layout_contents(
        &containing_block_for_children,
        collapsible_with_parent_start_margin,
    );
    if let Some(collapsing_context) = &mut collapsing_context {
        if collapsible_with_parent_start_margin.0 {
            collapsing_context
                .start
                .adjoin_assign(&flow_children.collapsing_context.start);
        }
    }
    let collapsible_with_parent_end_margin =
        matches!(block_level_kind, BlockLevelKind::SameFormattingContextBlock)
            && pb.block_end == Length::zero()
            && block_size == LengthOrAuto::Auto;
    let collapsed_end_margin = collapsing_context
        .as_mut()
        .map(|context| &mut context.end)
        .filter(|_| collapsible_with_parent_end_margin);
    if let Some(collapsed_end_margin) = collapsed_end_margin {
        collapsed_end_margin.adjoin_assign(&flow_children.collapsing_context.end);
    } else {
        flow_children.block_size += flow_children.collapsing_context.end.solve();
    }
    let relative_adjustement = relative_adjustement(style, inline_size, block_size);
    let block_size = block_size.auto_is(|| flow_children.block_size);
    let content_rect = Rect {
        start_corner: Vec2 {
            block: pb.block_start + relative_adjustement.block + initial_margin_block_start,
            inline: pb.inline_start + relative_adjustement.inline + margin.inline_start,
        },
        size: Vec2 {
            block: block_size,
            inline: inline_size,
        },
    };
    if style.box_.position.is_relatively_positioned() {
        AbsolutelyPositionedFragment::in_positioned_containing_block(
            &flow_children.absolutely_positioned_fragments,
            &mut flow_children.fragments,
            &content_rect.size,
            &padding,
            containing_block_for_children.mode,
        )
    } else {
        absolutely_positioned_fragments.extend(flow_children.absolutely_positioned_fragments);
    };
    BoxFragment {
        style: style.clone(),
        children: flow_children.fragments,
        content_rect,
        padding,
        border,
        margin,
        collapsing_context,
    }
}
