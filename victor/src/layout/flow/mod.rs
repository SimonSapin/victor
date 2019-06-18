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
pub(super) use inline::*;

#[derive(Debug)]
pub(super) struct BlockFormattingContext {
    pub contents: BlockContainer,
    pub contains_floats: bool,
}

#[derive(Debug)]
pub(super) enum BlockContainer {
    BlockLevelBoxes(Vec<Arc<BlockLevelBox>>),
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
    pub(super) fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
        placement_state: &mut PlacementState,
    ) -> Vec<Fragment> {
        let mut float_context;
        let float_context = if self.contains_floats {
            float_context = FloatContext::new();
            Some(&mut float_context)
        } else {
            None
        };
        placement_state.next_in_flow_margin_collapses_with_parent_start_margin = false;
        let fragments = self.contents.layout(
            containing_block,
            tree_rank,
            absolutely_positioned_fragments,
            placement_state,
            float_context,
        );
        placement_state.commit_current_margin();
        fragments
    }
}

impl BlockContainer {
    fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
        placement_state: &mut PlacementState,
        float_context: Option<&mut FloatContext>,
    ) -> Vec<Fragment> {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => layout_block_level_children(
                child_boxes,
                containing_block,
                tree_rank,
                absolutely_positioned_fragments,
                placement_state,
                float_context,
            ),
            BlockContainer::InlineFormattingContext(ifc) => ifc.layout(
                containing_block,
                tree_rank,
                absolutely_positioned_fragments,
                placement_state,
            ),
        }
    }
}

fn layout_block_level_children<'a>(
    child_boxes: &'a [Arc<BlockLevelBox>],
    containing_block: &ContainingBlock,
    tree_rank: usize,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    placement_state: &mut PlacementState,
    float_context: Option<&mut FloatContext>,
) -> Vec<Fragment> {
    let abspos_so_far = absolutely_positioned_fragments.len();
    let mut fragments: Vec<_>;
    if let Some(float_context) = float_context {
        // Because floats are involved, we do layout for this block formatting context
        // in tree order without parallelism. This enables mutable access
        // to a `FloatContext` that tracks every float encountered so far (again in tree order).
        fragments = child_boxes
            .iter()
            .enumerate()
            .map(|(tree_rank, box_)| {
                box_.layout(
                    containing_block,
                    tree_rank,
                    absolutely_positioned_fragments,
                    placement_state,
                    Some(float_context),
                )
            })
            .collect()
    } else {
        fragments = child_boxes
            .par_iter()
            .enumerate()
            .mapfold_reduce_into(
                absolutely_positioned_fragments,
                |abspos_fragments, (tree_rank, box_)| {
                    box_.layout(
                        containing_block,
                        tree_rank,
                        abspos_fragments,
                        &mut PlacementState::collapsible(),
                        /* float_context = */ None,
                    )
                },
                |left_abspos_fragments, mut right_abspos_fragments| {
                    left_abspos_fragments.append(&mut right_abspos_fragments);
                },
            )
            .collect();
        for fragment in &mut fragments {
            placement_state.place_block_level_fragment(fragment);
        }
    }

    adjust_static_positions(
        &mut absolutely_positioned_fragments[abspos_so_far..],
        &mut fragments,
        tree_rank,
    );

    fragments
}

impl BlockLevelBox {
    fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
        placement_state: &mut PlacementState,
        float_context: Option<&mut FloatContext>,
    ) -> Fragment {
        match self {
            BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                layout_in_flow_non_replaced_block_level(
                    containing_block,
                    absolutely_positioned_fragments,
                    style,
                    placement_state,
                    |containing_block, nested_abspos, nested_placement_state| {
                        contents.layout(
                            containing_block,
                            tree_rank,
                            nested_abspos,
                            nested_placement_state,
                            float_context,
                        )
                    },
                )
            }
            BlockLevelBox::Independent { style, contents } => match contents.as_replaced() {
                Ok(replaced) => {
                    // FIXME
                    match *replaced {}
                }
                Err(contents) => layout_in_flow_non_replaced_block_level(
                    containing_block,
                    absolutely_positioned_fragments,
                    style,
                    placement_state,
                    |containing_block, nested_abspos, nested_placement_state| {
                        contents.layout(
                            containing_block,
                            tree_rank,
                            nested_abspos,
                            nested_placement_state,
                        )
                    },
                ),
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
    placement_state: &mut PlacementState,
    layout_contents: impl FnOnce(
        &ContainingBlock,
        &mut Vec<AbsolutelyPositionedFragment<'a>>,
        &mut PlacementState,
    ) -> Vec<Fragment>,
) -> Fragment {
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
    let mut placement_state_for_children = PlacementState {
        next_in_flow_margin_collapses_with_parent_start_margin: pb.block_start == Length::zero(),
        start_margin: CollapsedMargin::new(margin.block_start),
        current_margin: CollapsedMargin::zero(),
        current_block_direction_position: Length::zero(),
    };
    let mut nested_abspos = vec![];
    let mut children = layout_contents(
        &containing_block_for_children,
        if style.box_.position.is_relatively_positioned() {
            &mut nested_abspos
        } else {
            absolutely_positioned_fragments
        },
        &mut placement_state_for_children,
    );

    let this_end_margin_can_collapse_with_children =
        (pb.block_end, block_size) == (Length::zero(), LengthOrAuto::Auto);

    if !this_end_margin_can_collapse_with_children {
        placement_state_for_children.commit_current_margin();
    }
    placement_state_for_children
        .current_margin
        .adjoin_assign(&CollapsedMargin::new(margin.block_end));
    let block_margins_collapsed_with_children = CollapsedBlockMargins {
        collapsed_through: placement_state_for_children
            .next_in_flow_margin_collapses_with_parent_start_margin
            && this_end_margin_can_collapse_with_children,
        start: placement_state_for_children.start_margin,
        end: placement_state_for_children.current_margin,
    };
    let relative_adjustement = relative_adjustement(style, inline_size, block_size);
    let block_size =
        block_size.auto_is(|| placement_state_for_children.current_block_direction_position);
    let content_rect = Rect {
        start_corner: Vec2 {
            block: pb.block_start + relative_adjustement.block,
            inline: pb.inline_start + relative_adjustement.inline + margin.inline_start,
        },
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
    }
    let mut fragment = Fragment::Box(BoxFragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
        block_margins_collapsed_with_children,
    });
    placement_state.place_block_level_fragment(&mut fragment);
    fragment
}

pub(super) struct PlacementState {
    next_in_flow_margin_collapses_with_parent_start_margin: bool,
    start_margin: CollapsedMargin,
    current_margin: CollapsedMargin,
    pub current_block_direction_position: Length,
}

impl PlacementState {
    pub fn root() -> Self {
        Self {
            next_in_flow_margin_collapses_with_parent_start_margin: false,
            start_margin: CollapsedMargin::zero(),
            current_margin: CollapsedMargin::zero(),
            current_block_direction_position: Length::zero(),
        }
    }

    pub fn collapsible() -> Self {
        Self {
            next_in_flow_margin_collapses_with_parent_start_margin: true,
            start_margin: CollapsedMargin::zero(),
            current_margin: CollapsedMargin::zero(),
            current_block_direction_position: Length::zero(),
        }
    }

    pub fn commit_current_margin(&mut self) {
        self.next_in_flow_margin_collapses_with_parent_start_margin = false;
        self.current_block_direction_position += self.current_margin.solve();
        self.current_margin = CollapsedMargin::zero();
    }

    fn place_block_level_fragment(&mut self, fragment: &mut Fragment) {
        match fragment {
            Fragment::Box(fragment) => {
                let fragment_block_margins = &fragment.block_margins_collapsed_with_children;
                let fragment_block_size = fragment.padding.block_sum()
                    + fragment.border.block_sum()
                    + fragment.content_rect.size.block;

                if self.next_in_flow_margin_collapses_with_parent_start_margin {
                    assert_eq!(self.current_margin.solve(), Length::zero());
                    self.start_margin
                        .adjoin_assign(&fragment_block_margins.start);
                    if fragment_block_margins.collapsed_through {
                        self.start_margin.adjoin_assign(&fragment_block_margins.end);
                        return;
                    }
                    self.next_in_flow_margin_collapses_with_parent_start_margin = false;
                } else {
                    self.current_margin
                        .adjoin_assign(&fragment_block_margins.start);
                }
                fragment.content_rect.start_corner.block +=
                    self.current_margin.solve() + self.current_block_direction_position;
                if fragment_block_margins.collapsed_through {
                    self.current_margin
                        .adjoin_assign(&fragment_block_margins.end);
                    return;
                }
                self.current_block_direction_position +=
                    self.current_margin.solve() + fragment_block_size;
                self.current_margin = fragment_block_margins.end;
            }
            Fragment::Anonymous(fragment) => {
                // FIXME(nox): Margin collapsing for hypothetical boxes of
                // abspos elements is probably wrong.
                assert!(fragment.children.is_empty());
                assert_eq!(fragment.rect.size.block, Length::zero());
                fragment.rect.start_corner.block += self.current_block_direction_position;
            }
            _ => unreachable!(),
        }
    }
}
