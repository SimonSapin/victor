mod abspos;
mod boxes;
pub(crate) mod fragments;

use self::abspos::*;
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

struct ContainingBlock {
    inline_size: Length,
    block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

impl BlockFormattingContext {
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
                absolutely_positioned_box.layout(tree_rank, absolutely_positioned_fragments)
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
