mod boxes;
pub(crate) mod fragments;

use self::boxes::*;
use self::fragments::*;
use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::values::{Direction, LengthOrPercentage, LengthOrPercentageOrAuto, WritingMode};
use crate::style::ComputedValues;
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

    let (fragments, _) = box_tree.layout_into_absolute_containing_block(
        &initial_containing_block,
        &initial_containing_block_padding,
    );
    fragments
}

#[derive(Debug)]
struct AbsolutelyPositionedFragment<'box_> {
    absolutely_positioned_box: &'box_ AbsolutelyPositionedBox,

    /// The rank of the child from which this absolutely positioned fragment
    /// came from, when doing the layout of a block container. Used to compute
    /// static positions when going up the tree.
    tree_rank: usize,

    inline_start: AbsoluteBoxOffsets<LengthOrPercentage>,
    inline_size: Option<LengthOrPercentage>,

    block_start: AbsoluteBoxOffsets<LengthOrPercentage>,
    block_size: Option<LengthOrPercentage>,
}

#[derive(Clone, Copy, Debug)]
enum AbsoluteBoxOffsets<NonStatic> {
    StaticStart { start: Length },
    Start { start: NonStatic },
    End { end: NonStatic },
    Both { start: NonStatic, end: NonStatic },
}

struct AbsoluteContainingBlock {
    size: Vec2<Length>,
    padding_start: Vec2<Length>,
    mode: (WritingMode, Direction),
}

struct ContainingBlock {
    inline_size: Length,
    block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

impl BlockFormattingContext {
    fn layout_into_absolute_containing_block(
        &self,
        containing_block: &ContainingBlock,
        containing_block_padding: &Sides<Length>,
    ) -> (Vec<Fragment>, Length) {
        let (mut fragments, absolutely_positioned_fragments, block_size) =
            self.layout(containing_block, 0);
        let absolute_containing_block = AbsoluteContainingBlock {
            size: Vec2 {
                inline: containing_block.inline_size + containing_block_padding.inline_sum(),
                block: block_size + containing_block_padding.block_sum(),
            },
            padding_start: Vec2 {
                inline: containing_block_padding.inline_start,
                block: containing_block_padding.block_start,
            },
            mode: containing_block.mode,
        };
        // FIXME(nox): Should we do that with parallel iterators, too? It's
        // trivial to do so at the very least.
        fragments.extend(
            absolutely_positioned_fragments
                .iter()
                .map(|f| f.layout(&absolute_containing_block)),
        );
        (fragments, block_size)
    }

    fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        self.0.layout(containing_block, tree_rank)
    }
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
            BlockContainer::InlineFormattingContext(ifc) => {
                let (child_fragments, block_size) = ifc.layout(containing_block);
                // FIXME(nox): Handle abspos in inline.
                (child_fragments, vec![].into(), block_size)
            }
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
            BlockLevelBox::AbsolutelyPositionedBox(absolutely_positioned_box) => {
                absolutely_positioned_child(
                    tree_rank,
                    absolutely_positioned_fragments,
                    absolutely_positioned_box,
                )
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
    let (children, abspos_children, content_block_size) =
        contents.layout(&containing_block_for_children, tree_rank);
    let block_size = block_size.unwrap_or(content_block_size);
    let content_rect = Rect {
        start_corner: pbm.start_corner(),
        size: Vec2 {
            block: block_size,
            inline: inline_size,
        },
    };
    let fragment = Fragment::Box(BoxFragment {
        style: style.clone(),
        children,
        content_rect,
        padding,
        border,
        margin,
    });
    absolutely_positioned_fragments.extend(abspos_children);
    fragment
}

fn absolutely_positioned_child<'a>(
    tree_rank: usize,
    absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    absolutely_positioned_box: &'a AbsolutelyPositionedBox,
) -> Fragment {
    let style = &absolutely_positioned_box.style;
    let box_offsets = style.box_offsets();
    let box_size = style.box_size();

    let inline_size = box_size.inline.non_auto();
    let block_size = box_size.block.non_auto();

    fn absolute_box_offsets(
        start: LengthOrPercentageOrAuto,
        end: LengthOrPercentageOrAuto,
    ) -> AbsoluteBoxOffsets<LengthOrPercentage> {
        match (start.non_auto(), end.non_auto()) {
            (None, None) => AbsoluteBoxOffsets::StaticStart {
                start: Length::zero(),
            },
            (Some(start), Some(end)) => AbsoluteBoxOffsets::Both { start, end },
            (None, Some(end)) => AbsoluteBoxOffsets::End { end },
            (Some(start), None) => AbsoluteBoxOffsets::Start { start },
        }
    }

    let inline_start = absolute_box_offsets(box_offsets.inline_start, box_offsets.inline_end);
    let block_start = absolute_box_offsets(box_offsets.block_start, box_offsets.block_end);

    let fragment = Fragment::Box(BoxFragment::zero_sized(style.clone()));

    absolutely_positioned_fragments.push(AbsolutelyPositionedFragment {
        absolutely_positioned_box,
        tree_rank,
        inline_start,
        inline_size,
        block_start,
        block_size,
    });

    fragment
}

impl<'a> AbsolutelyPositionedFragment<'a> {
    fn layout(&self, absolute_containing_block: &AbsoluteContainingBlock) -> Fragment {
        let style = &self.absolutely_positioned_box.style;
        let cbis = absolute_containing_block.size.inline;
        let cbbs = absolute_containing_block.size.block;
        let zero = Length::zero();

        let padding = style.padding().map(|v| v.percentage_relative_to(cbis));
        let border = style.border_width().map(|v| v.percentage_relative_to(cbis));
        let pb = &padding + &border;

        let computed_margin = style
            .margin()
            .map(|v| v.non_auto().map(|v| v.percentage_relative_to(cbis)));

        enum Anchor {
            Start(Length),
            End(Length),
        }

        fn solve_axis(
            containing_size: Length,
            containing_padding_start: Length,
            padding_border_sum: Length,
            computed_margin_start: Option<Length>,
            computed_margin_end: Option<Length>,
            solve_margins: impl FnOnce(Length) -> (Length, Length),
            box_offsets: AbsoluteBoxOffsets<LengthOrPercentage>,
            size: Option<LengthOrPercentage>,
        ) -> (Anchor, Option<Length>, Length, Length) {
            let zero = Length::zero();
            let size = size.map(|v| v.percentage_relative_to(containing_size));
            match box_offsets {
                AbsoluteBoxOffsets::StaticStart { start } => (
                    Anchor::Start(start + containing_padding_start),
                    size,
                    computed_margin_start.unwrap_or(zero),
                    computed_margin_end.unwrap_or(zero),
                ),
                AbsoluteBoxOffsets::Start { start } => (
                    Anchor::Start(start.percentage_relative_to(containing_size)),
                    size,
                    computed_margin_start.unwrap_or(zero),
                    computed_margin_end.unwrap_or(zero),
                ),
                AbsoluteBoxOffsets::End { end } => (
                    Anchor::End(end.percentage_relative_to(containing_size)),
                    size,
                    computed_margin_start.unwrap_or(zero),
                    computed_margin_end.unwrap_or(zero),
                ),
                AbsoluteBoxOffsets::Both { start, end } => {
                    let start = start.percentage_relative_to(containing_size);
                    let end = end.percentage_relative_to(containing_size);

                    let mut margin_start = computed_margin_start.unwrap_or(zero);
                    let mut margin_end = computed_margin_end.unwrap_or(zero);

                    let size = if let Some(size) = size {
                        let margins = containing_size - start - end - padding_border_sum - size;
                        match (computed_margin_start, computed_margin_end) {
                            (None, None) => {
                                let (s, e) = solve_margins(margins);
                                margin_start = s;
                                margin_end = e;
                            }
                            (None, Some(end)) => {
                                margin_start = margins - end;
                            }
                            (Some(start), None) => {
                                margin_end = margins - start;
                            }
                            (Some(_), Some(_)) => {}
                        }
                        size
                    } else {
                        // FIXME(nox): What happens if that is negative?
                        containing_size
                            - start
                            - end
                            - padding_border_sum
                            - margin_start
                            - margin_end
                    };
                    (Anchor::Start(start), Some(size), margin_start, margin_end)
                }
            }
        }

        let (inline_anchor, inline_size, margin_inline_start, margin_inline_end) = solve_axis(
            cbis,
            absolute_containing_block.padding_start.inline,
            pb.inline_sum(),
            computed_margin.inline_start,
            computed_margin.inline_end,
            |margins| {
                if margins.px >= 0. {
                    (margins / 2., margins / 2.)
                } else {
                    (zero, margins)
                }
            },
            self.inline_start,
            self.inline_size,
        );

        let (block_anchor, block_size, margin_block_start, margin_block_end) = solve_axis(
            cbis,
            absolute_containing_block.padding_start.block,
            pb.block_sum(),
            computed_margin.block_start,
            computed_margin.block_end,
            |margins| (margins / 2., margins / 2.),
            self.block_start,
            self.block_size,
        );

        let margin = Sides {
            inline_start: margin_inline_start,
            inline_end: margin_inline_end,
            block_start: margin_block_start,
            block_end: margin_block_end,
        };

        let inline_size = inline_size.unwrap_or_else(|| {
            let available_size = match inline_anchor {
                Anchor::Start(start) => cbis - start - pb.inline_sum() - margin.inline_sum(),
                Anchor::End(end) => cbis - end - pb.inline_sum() - margin.inline_sum(),
            };

            // FIXME(nox): shrink-to-fit.
            available_size
        });

        let containing_block_for_children = ContainingBlock {
            inline_size,
            block_size,
            mode: style.writing_mode(),
        };
        // https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
        assert_eq!(
            absolute_containing_block.mode, containing_block_for_children.mode,
            "Mixed writing modes are not supported yet"
        );
        let (children, block_size) = self
            .absolutely_positioned_box
            .contents
            .layout_into_absolute_containing_block(&containing_block_for_children, &padding);

        let inline_start = match inline_anchor {
            Anchor::Start(start) => start,
            Anchor::End(end) => cbbs - end - pb.inline_end - margin.inline_end - inline_size,
        };

        let block_start = match block_anchor {
            Anchor::Start(start) => start,
            Anchor::End(end) => cbbs - end - pb.block_end - margin.block_end - block_size,
        };

        let content_rect = Rect {
            start_corner: Vec2 {
                inline: inline_start - absolute_containing_block.padding_start.inline,
                block: block_start - absolute_containing_block.padding_start.block,
            },
            size: Vec2 {
                inline: inline_size,
                block: block_size,
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
}

struct InlineFormattingContextLayoutState<'a> {
    remaining_boxes: std::slice::Iter<'a, InlineLevelBox>,
    fragments_so_far: Vec<Fragment>,
    max_block_size_of_fragments_so_far: Length,
}

struct PartialInlineBoxFragment<'a> {
    fragment: BoxFragment,
    last_fragment: bool,
    saved_state: InlineFormattingContextLayoutState<'a>,
}

impl InlineFormattingContext {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<Fragment>, Length) {
        let mut partial_inline_boxes_stack = Vec::new();
        let mut inline_position = Length::zero();

        let mut state = InlineFormattingContextLayoutState {
            remaining_boxes: self.inline_level_boxes.iter(),
            fragments_so_far: Vec::with_capacity(self.inline_level_boxes.len()),
            max_block_size_of_fragments_so_far: Length::zero(),
        };
        loop {
            if let Some(child) = state.remaining_boxes.next() {
                match child {
                    InlineLevelBox::InlineBox(inline) => partial_inline_boxes_stack.push(
                        inline.start_layout(containing_block, &mut inline_position, &mut state),
                    ),
                    InlineLevelBox::TextRun(id) => {
                        self.text_runs[id.0].layout(&mut inline_position, &mut state)
                    }
                }
            } else
            // Reached the end of state.remaining_boxes
            if let Some(partial) = partial_inline_boxes_stack.pop() {
                partial.finish_layout(&mut inline_position, &mut state)
            } else {
                return (
                    state.fragments_so_far,
                    state.max_block_size_of_fragments_so_far,
                )
            }
        }
    }
}

impl InlineBox {
    fn start_layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        inline_position: &mut Length,
        ifc_state: &mut InlineFormattingContextLayoutState<'a>,
    ) -> PartialInlineBoxFragment<'a> {
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
        let content_rect = Rect {
            start_corner: Vec2 {
                block: padding.block_start + border.block_start + margin.block_start,
                inline: *inline_position,
            },
            size: Vec2 {
                block: Length::zero(),
                inline: Length::zero(),
            },
        };
        let fragment = BoxFragment {
            style,
            content_rect,
            padding,
            border,
            margin,
            children: Vec::new(),
        };
        PartialInlineBoxFragment {
            fragment,
            last_fragment: self.last_fragment,
            saved_state: std::mem::replace(
                ifc_state,
                InlineFormattingContextLayoutState {
                    remaining_boxes: self.children.iter(),
                    fragments_so_far: Vec::with_capacity(self.children.len()),
                    max_block_size_of_fragments_so_far: Length::zero(),
                },
            ),
        }
    }
}

impl<'a> PartialInlineBoxFragment<'a> {
    fn finish_layout(
        mut self,
        inline_position: &mut Length,
        ifc_state: &mut InlineFormattingContextLayoutState<'a>,
    ) {
        let mut fragment = self.fragment;
        fragment.content_rect.size = Vec2 {
            inline: *inline_position - fragment.content_rect.start_corner.inline,
            block: ifc_state.max_block_size_of_fragments_so_far,
        };
        if self.last_fragment {
            *inline_position += fragment.padding.inline_end
                + fragment.border.inline_end
                + fragment.margin.inline_end;
        } else {
            fragment.padding.inline_end = Length::zero();
            fragment.border.inline_end = Length::zero();
            fragment.margin.inline_end = Length::zero();
        }
        self.saved_state
            .max_block_size_of_fragments_so_far
            .max_assign(
                fragment.content_rect.size.block
                    + fragment.padding.block_sum()
                    + fragment.border.block_sum()
                    + fragment.margin.block_sum(),
            );
        fragment.children = ifc_state.fragments_so_far.take();
        debug_assert!(ifc_state.remaining_boxes.as_slice().is_empty());
        *ifc_state = self.saved_state;
        ifc_state.fragments_so_far.push(Fragment::Box(fragment));
    }
}

impl TextRun {
    fn layout(
        &self,
        inline_position: &mut Length,
        ifc_state: &mut InlineFormattingContextLayoutState,
    ) {
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
        ifc_state
            .max_block_size_of_fragments_so_far
            .max_assign(line_height);
        ifc_state
            .fragments_so_far
            .push(Fragment::Text(TextFragment {
                parent_style,
                content_rect,
                // FIXME: keep Arc<ShapedSegment> instead of ShapedSegment,
                // to make this clone cheaper?
                text: self.segment.clone(),
            }));
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
