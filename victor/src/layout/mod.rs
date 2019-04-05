mod boxes;
pub(crate) mod fragments;

use self::boxes::*;
use self::fragments::*;
use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::values::{Direction, LengthOrPercentage, LengthOrPercentageOrAuto, WritingMode};
use crate::style::ComputedValues;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelExtend,
    ParallelIterator,
};
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
        inline_start_from_absolute_containing_block: Length::zero(),
        inline_size,
        block_size: Some(Length {
            px: viewport.height,
        }),
        mode: (WritingMode::HorizontalTb, Direction::Ltr),
    };

    let (mut fragments, absolutely_positioned_fragments, _) =
        box_tree.layout(&initial_containing_block, &initial_containing_block, 0);
    fragments.extend(
        absolutely_positioned_fragments
            .into_iter()
            .map(|fragment| Fragment::Box(fragment.contents)),
    );
    fragments
}

struct AbsolutelyPositionedFragment {
    index: usize,
    uses_static_block_position: bool,
    contents: BoxFragment,
}

struct ContainingBlock {
    inline_start_from_absolute_containing_block: Length,
    inline_size: Length,
    block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

impl BlockFormattingContext {
    fn layout(
        &self,
        absolute_containing_block: &ContainingBlock,
        containing_block: &ContainingBlock,
        index: usize,
    ) -> (Vec<Fragment>, FlatVec<AbsolutelyPositionedFragment>, Length) {
        self.0
            .layout(absolute_containing_block, containing_block, index)
    }
}

impl BlockContainer {
    fn layout(
        &self,
        absolute_containing_block: &ContainingBlock,
        containing_block: &ContainingBlock,
        index: usize,
    ) -> (Vec<Fragment>, FlatVec<AbsolutelyPositionedFragment>, Length) {
        match self {
            BlockContainer::BlockLevelBoxes(child_boxes) => {
                let (mut child_fragments, mut absolutely_positioned_fragments) = child_boxes
                    .par_iter()
                    .enumerate()
                    .map(|(index, child)| {
                        child.layout(absolute_containing_block, containing_block, index)
                    })
                    .unzip::<_, _, Vec<_>, FlatVec<_>>();

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

                for abspos_fragment in &mut absolutely_positioned_fragments {
                    if abspos_fragment.uses_static_block_position {
                        let child_fragment = match &child_fragments[abspos_fragment.index] {
                            Fragment::Box(b) => b,
                            _ => unreachable!(),
                        };
                        abspos_fragment.contents.content_rect.start_corner.block +=
                            child_fragment.content_rect.start_corner.block;
                    }
                    abspos_fragment.index = index;
                }

                (
                    child_fragments,
                    absolutely_positioned_fragments,
                    block_size,
                )
            }
            BlockContainer::InlineFormattingContext(ifc) => {
                let (child_fragments, block_size) = ifc.layout(containing_block);
                // FIXME(nox): Handle abspos in inline.
                (child_fragments, vec![].into(), block_size)
            }
        }
    }
}

impl BlockLevelBox {
    fn layout(
        &self,
        absolute_containing_block: &ContainingBlock,
        containing_block: &ContainingBlock,
        index: usize,
    ) -> (Fragment, FlatVec<AbsolutelyPositionedFragment>) {
        match self {
            BlockLevelBox::SameFormattingContextBlock { style, contents } => {
                same_formatting_context_block(
                    absolute_containing_block,
                    containing_block,
                    index,
                    style,
                    contents,
                )
            }
            BlockLevelBox::AbsolutelyPositionedBox { style, contents } => {
                absolutely_positioned_box(
                    absolute_containing_block,
                    containing_block,
                    index,
                    style,
                    contents,
                )
            }
        }
    }
}

fn same_formatting_context_block(
    absolute_containing_block: &ContainingBlock,
    containing_block: &ContainingBlock,
    index: usize,
    style: &Arc<ComputedValues>,
    contents: &boxes::BlockContainer,
) -> (Fragment, FlatVec<AbsolutelyPositionedFragment>) {
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
        inline_start_from_absolute_containing_block: pbm.inline_start
            + containing_block.inline_start_from_absolute_containing_block,
        inline_size,
        block_size,
        mode: style.writing_mode(),
    };
    // https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
    assert_eq!(
        containing_block.mode, containing_block_for_children.mode,
        "Mixed writing modes are not supported yet"
    );
    let (children, absolutely_positioned_fragments, content_block_size) = contents.layout(
        absolute_containing_block,
        &containing_block_for_children,
        index,
    );
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
    (fragment, absolutely_positioned_fragments)
}

fn absolutely_positioned_box(
    absolute_containing_block: &ContainingBlock,
    containing_block: &ContainingBlock,
    index: usize,
    style: &Arc<ComputedValues>,
    contents: &BlockFormattingContext,
) -> (Fragment, FlatVec<AbsolutelyPositionedFragment>) {
    let cbis = absolute_containing_block.inline_size;
    let padding = style.padding().map(|v| v.percentage_relative_to(cbis));
    let border = style.border_width().map(|v| v.percentage_relative_to(cbis));
    let pb = &padding + &border;
    let box_size = style.box_size();

    let computed_physical_margin = style
        .physical_margin()
        .map(|v| v.non_auto().map(|v| v.percentage_relative_to(cbis)));
    let computed_margin = style
        .margin()
        .map(|v| v.non_auto().map(|v| v.percentage_relative_to(cbis)));

    let computed_inline_size = box_size
        .inline
        .non_auto()
        .map(|v| v.percentage_relative_to(cbis));
    let computed_block_size = box_size.block.non_auto().and_then(|b| match b {
        LengthOrPercentage::Length(l) => Some(l),
        LengthOrPercentage::Percentage(p) => {
            absolute_containing_block.block_size.map(|cbbs| cbbs * p)
        }
    });

    struct Solution {
        margin_start: Length,
        margin_end: Length,
        strategy: Strategy,
    }

    enum Strategy {
        FromStart {
            start: Option<Length>,
            size: Option<Length>,
        },
        FromEnd {
            end: Length,
        },
    }

    fn solve_axis(
        containing_block_inline_size: Length,
        physical_start: Option<Length>,
        physical_end: Option<Length>,
        computed_margin_start: Option<Length>,
        computed_margin_end: Option<Length>,
        solve_margins: impl FnOnce(Length) -> (Length, Length),
        padding_border_sum: Length,
        size: Option<Length>,
    ) -> Solution {
        let cbis_minus_pb = containing_block_inline_size - padding_border_sum;
        let zero = Length::zero();

        let mut margin_start = computed_margin_start.unwrap_or(zero);
        let mut margin_end = computed_margin_end.unwrap_or(zero);

        let strategy = match (physical_start, size, physical_end) {
            (start, size, None) => Strategy::FromStart { start, size },
            (Some(start), Some(size), Some(end)) => {
                let margins = cbis_minus_pb - start - size - end;

                match (computed_margin_start, computed_margin_end) {
                    (None, None) => {
                        let (s, e) = solve_margins(margins);
                        margin_start = s;
                        margin_end = e;
                    }
                    (None, Some(end)) => {
                        margin_start = margins - end;
                        margin_end = end;
                    }
                    (Some(start), _) => {
                        margin_start = start;
                        margin_end = margins - start;
                    }
                }

                Strategy::FromStart {
                    start: Some(start),
                    size: Some(size),
                }
            }
            (None, None, Some(end)) => Strategy::FromEnd { end },
            (None, Some(size), Some(end)) => {
                let start = cbis_minus_pb - size - end - margin_start - margin_end;
                Strategy::FromStart {
                    start: Some(start),
                    size: Some(size),
                }
            }
            (Some(start), None, Some(end)) => {
                // FIXME(nox): Wait, what happens when that is negative?
                let size = cbis_minus_pb - start - end - margin_start - margin_end;
                Strategy::FromStart {
                    start: Some(start),
                    size: Some(size),
                }
            }
        };

        Solution {
            margin_start,
            margin_end,
            strategy,
        }
    }

    // https://drafts.csswg.org/css2/visudet.html#abs-non-replaced-width
    let inline_solution = solve_axis(
        cbis,
        computed_physical_margin.inline_start,
        computed_physical_margin.inline_end,
        computed_margin.inline_start,
        computed_margin.inline_end,
        |margins| {
            if margins.px >= 0. {
                (margins / 2., margins / 2.)
            } else {
                (Length::zero(), margins)
            }
        },
        pb.inline_sum(),
        computed_inline_size,
    );

    let inline_size;
    let inline_start;
    match inline_solution.strategy {
        Strategy::FromStart { start, size } => {
            inline_start =
                start.unwrap_or(containing_block.inline_start_from_absolute_containing_block);
            inline_size = size.unwrap_or_else(|| {
                let available_size =
                    cbis - inline_start - inline_solution.margin_start - inline_solution.margin_end;
                // FIXME(nox): shrink-to-fit inline size.
                available_size
            });
        }
        Strategy::FromEnd { end } => {
            inline_start = Length::zero();
            let available_size =
                cbis - end - inline_solution.margin_start - inline_solution.margin_end;
            // FIXME(nox): shrink-to-fit inline size.
            inline_size = available_size;
        }
    }

    // https://drafts.csswg.org/css2/visudet.html#abs-non-replaced-height
    let block_solution = solve_axis(
        cbis,
        computed_physical_margin.block_start,
        computed_physical_margin.block_end,
        computed_margin.block_start,
        computed_margin.block_end,
        |margins| (margins / 2., margins / 2.),
        pb.block_sum(),
        computed_block_size,
    );

    let block_size = match block_solution.strategy {
        Strategy::FromStart { size, .. } => size,
        Strategy::FromEnd { .. } => None,
    };

    let containing_block_for_children = ContainingBlock {
        inline_start_from_absolute_containing_block: Length::zero(),
        inline_size,
        block_size,
        mode: style.writing_mode(),
    };

    // https://drafts.csswg.org/css-writing-modes/#orthogonal-flows
    assert_eq!(
        absolute_containing_block.mode, containing_block.mode,
        "Mixed writing modes are not supported yet"
    );
    assert_eq!(
        containing_block.mode, containing_block_for_children.mode,
        "Mixed writing modes are not supported yet"
    );

    let (mut children, absolutely_positioned_fragments, content_block_size) = contents.layout(
        &containing_block_for_children,
        &containing_block_for_children,
        0,
    );
    children.extend(
        absolutely_positioned_fragments
            .into_iter()
            .map(|fragment| Fragment::Box(fragment.contents)),
    );

    let block_size = block_size.unwrap_or(content_block_size);
    let (block_start, uses_static_block_position) = match block_solution.strategy {
        Strategy::FromStart { start: None, .. } => (Length::zero(), true),
        Strategy::FromStart {
            start: Some(start), ..
        } => (start, false),
        Strategy::FromEnd { end } => (cbis - end - block_size, false),
    };

    let margin_start_corner = Vec2 {
        block: block_start,
        inline: inline_start,
    };
    let margin = Sides {
        inline_start: inline_solution.margin_start,
        inline_end: inline_solution.margin_end,
        block_start: block_solution.margin_start,
        block_end: block_solution.margin_end,
    };
    let pbm = &pb + &margin;

    let content_rect = Rect {
        start_corner: &margin_start_corner + &pbm.start_corner(),
        size: Vec2 {
            block: block_size,
            inline: inline_size,
        },
    };

    let fragment = Fragment::Box(BoxFragment::zero_sized(style.clone()));

    let absolutely_positioned_fragment = AbsolutelyPositionedFragment {
        index,
        uses_static_block_position,
        contents: BoxFragment {
            style: style.clone(),
            children,
            content_rect,
            padding,
            border,
            margin,
        },
    };

    (fragment, vec![absolutely_positioned_fragment].into())
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

struct FlatVec<T>(Vec<T>);

impl<T> Default for FlatVec<T> {
    fn default() -> Self {
        Self(vec![])
    }
}

impl<T> From<Vec<T>> for FlatVec<T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec)
    }
}

impl<'a, T> IntoIterator for FlatVec<T>
where
    T: Send + 'a,
{
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut FlatVec<T>
where
    T: Send + 'a,
{
    type Item = &'a mut T;
    type IntoIter = <&'a mut Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&mut self.0).into_iter()
    }
}

impl<T> IntoParallelIterator for FlatVec<T>
where
    T: Send,
{
    type Iter = <Vec<T> as IntoParallelIterator>::Iter;
    type Item = T;

    fn into_par_iter(self) -> Self::Iter {
        self.0.into_par_iter()
    }
}

impl<T> ParallelExtend<Self> for FlatVec<T>
where
    T: Send,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = Self>,
    {
        self.0.par_extend(par_iter.into_par_iter().flatten());
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
