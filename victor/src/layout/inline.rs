use super::abspos::AbsolutelyPositionedFragment;
use super::boxes::{InlineBox, InlineFormattingContext, InlineLevelBox, TextRun};
use super::fragments::{BoxFragment, Fragment, TextFragment};
use super::{relative_adjustement, ContainingBlock, Take};
use crate::fonts::BITSTREAM_VERA_SANS;
use crate::geom::flow_relative::{Rect, Vec2};
use crate::geom::Length;
use crate::style::values::{Display, DisplayInside, DisplayOutside};
use crate::text::ShapedSegment;

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
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        let mut absolutely_positioned_fragments = Vec::new();
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
                    InlineLevelBox::TextRun(run) => run.layout(&mut inline_position, &mut state),
                    InlineLevelBox::OutOfFlowAbsolutelyPositionedBox(box_) => {
                        let initial_start_corner = match box_.style.specified_display {
                            Display::Other {
                                outside: DisplayOutside::Inline,
                                inside: DisplayInside::Flow,
                            } => Vec2 {
                                inline: inline_position,
                                // FIXME(nox): Line-breaking will make that incorrect.
                                block: Length::zero(),
                            },
                            Display::Other {
                                outside: DisplayOutside::Block,
                                inside: DisplayInside::Flow,
                            } => Vec2 {
                                inline: Length::zero(),
                                block: state.max_block_size_of_fragments_so_far,
                            },
                            Display::None => panic!("abspos box cannot be display:none"),
                        };
                        absolutely_positioned_fragments
                            .push(box_.layout(initial_start_corner, tree_rank));
                    }
                }
            } else
            // Reached the end of state.remaining_boxes
            if let Some(partial) = partial_inline_boxes_stack.pop() {
                partial.finish_layout(containing_block, &mut inline_position, &mut state)
            } else {
                return (
                    state.fragments_so_far,
                    absolutely_positioned_fragments,
                    state.max_block_size_of_fragments_so_far,
                );
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
            size: Vec2::zero(),
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
        containing_block: &ContainingBlock,
        inline_position: &mut Length,
        ifc_state: &mut InlineFormattingContextLayoutState<'a>,
    ) {
        let mut fragment = self.fragment;
        fragment.content_rect.size = Vec2 {
            inline: *inline_position - fragment.content_rect.start_corner.inline,
            block: ifc_state.max_block_size_of_fragments_so_far,
        };
        fragment.content_rect.start_corner += &relative_adjustement(
            &fragment.style,
            containing_block.inline_size,
            containing_block.block_size,
        );
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
        let mut text = ShapedSegment::new_with_naive_shaping(BITSTREAM_VERA_SANS.clone());
        text.append(self.text.chars()).unwrap();
        let inline_size = parent_style.font.font_size * text.advance_width;
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
                text,
            }));
    }
}
