use super::boxes::{AbsolutelyPositionedBox, BlockContainer};
use super::fragments::{BoxFragment, Fragment};
use super::ContainingBlock;
use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::values::{Direction, LengthOrPercentage, LengthOrPercentageOrAuto, WritingMode};

#[derive(Debug)]
pub(super) struct AbsolutelyPositionedFragment<'box_> {
    absolutely_positioned_box: &'box_ AbsolutelyPositionedBox,

    /// The rank of the child from which this absolutely positioned fragment
    /// came from, when doing the layout of a block container. Used to compute
    /// static positions when going up the tree.
    pub(super) tree_rank: usize,

    pub(super) inline_start: AbsoluteBoxOffsets<LengthOrPercentage>,
    inline_size: Option<LengthOrPercentage>,

    pub(super) block_start: AbsoluteBoxOffsets<LengthOrPercentage>,
    block_size: Option<LengthOrPercentage>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum AbsoluteBoxOffsets<NonStatic> {
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

impl BlockContainer {
    pub(super) fn layout_into_absolute_containing_block(
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
}

impl AbsolutelyPositionedBox {
    pub(super) fn layout<'a>(
        &'a self,
        initial_start_corner: Vec2<Length>,
        tree_rank: usize,
    ) -> AbsolutelyPositionedFragment {
        let style = &self.style;
        let box_offsets = style.box_offsets();
        let box_size = style.box_size();

        let inline_size = box_size.inline.non_auto();
        let block_size = box_size.block.non_auto();

        fn absolute_box_offsets(
            initial_static_start: Length,
            start: LengthOrPercentageOrAuto,
            end: LengthOrPercentageOrAuto,
        ) -> AbsoluteBoxOffsets<LengthOrPercentage> {
            match (start.non_auto(), end.non_auto()) {
                (None, None) => AbsoluteBoxOffsets::StaticStart {
                    start: initial_static_start,
                },
                (Some(start), Some(end)) => AbsoluteBoxOffsets::Both { start, end },
                (None, Some(end)) => AbsoluteBoxOffsets::End { end },
                (Some(start), None) => AbsoluteBoxOffsets::Start { start },
            }
        }

        let inline_start = absolute_box_offsets(
            initial_start_corner.inline,
            box_offsets.inline_start,
            box_offsets.inline_end,
        );
        let block_start = absolute_box_offsets(
            initial_start_corner.block,
            box_offsets.block_start,
            box_offsets.block_end,
        );

        AbsolutelyPositionedFragment {
            absolutely_positioned_box: self,
            tree_rank,
            inline_start,
            inline_size,
            block_start,
            block_size,
        }
    }
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
            .0
            .layout_into_absolute_containing_block(&containing_block_for_children, &padding);

        let inline_start = match inline_anchor {
            Anchor::Start(start) => start + pb.inline_start + margin.inline_start,
            Anchor::End(end) => cbbs - end - pb.inline_end - margin.inline_end - inline_size,
        };

        let block_start = match block_anchor {
            Anchor::Start(start) => start + pb.block_start + margin.block_start,
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
