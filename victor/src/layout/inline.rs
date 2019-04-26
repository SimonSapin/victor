use super::abspos::AbsolutelyPositionedFragment;
use super::boxes::{InlineBox, InlineFormattingContext, InlineLevelBox, TextRun};
use super::fragments::{BoxFragment, Fragment, TextFragment};
use super::{relative_adjustement, ContainingBlock, Take};
use crate::fonts::BITSTREAM_VERA_SANS;
use crate::geom::flow_relative::{Rect, Vec2};
use crate::geom::Length;
use crate::style::values::{Display, DisplayInside, DisplayOutside};
use crate::text::ShapedSegment;

struct InlineNestingLevelState<'box_tree> {
    remaining_boxes: std::slice::Iter<'box_tree, InlineLevelBox>,
    fragments_so_far: Vec<Fragment>,
    max_block_size_of_fragments_so_far: Length,
}

struct PartialInlineBoxFragment<'box_tree> {
    fragment: BoxFragment,
    last_fragment: bool,
    saved_state: InlineNestingLevelState<'box_tree>,
}

struct InlineFormattingContextState<'box_tree, 'cb> {
    containing_block: &'cb ContainingBlock,
    absolutely_positioned_fragments: Vec<AbsolutelyPositionedFragment<'box_tree>>,
    inline_position: Length,
    partial_inline_boxes_stack: Vec<PartialInlineBoxFragment<'box_tree>>,
    current_nesting_level: InlineNestingLevelState<'box_tree>,
}

impl InlineFormattingContext {
    pub(super) fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
    ) -> (Vec<Fragment>, Vec<AbsolutelyPositionedFragment>, Length) {
        let mut ifc = InlineFormattingContextState {
            containing_block,
            absolutely_positioned_fragments: Vec::new(),
            partial_inline_boxes_stack: Vec::new(),
            inline_position: Length::zero(),
            current_nesting_level: InlineNestingLevelState {
                remaining_boxes: self.inline_level_boxes.iter(),
                fragments_so_far: Vec::with_capacity(self.inline_level_boxes.len()),
                max_block_size_of_fragments_so_far: Length::zero(),
            },
        };
        loop {
            if let Some(child) = ifc.current_nesting_level.remaining_boxes.next() {
                match child {
                    InlineLevelBox::InlineBox(inline) => {
                        let partial = inline.start_layout(&mut ifc);
                        ifc.partial_inline_boxes_stack.push(partial)
                    }
                    InlineLevelBox::TextRun(run) => run.layout(&mut ifc),
                    InlineLevelBox::OutOfFlowAbsolutelyPositionedBox(box_) => {
                        let initial_start_corner = match box_.style.specified_display {
                            Display::Other {
                                outside: DisplayOutside::Inline,
                                inside: DisplayInside::Flow,
                            } => Vec2 {
                                inline: ifc.inline_position,
                                // FIXME(nox): Line-breaking will make that incorrect.
                                block: Length::zero(),
                            },
                            Display::Other {
                                outside: DisplayOutside::Block,
                                inside: DisplayInside::Flow,
                            } => Vec2 {
                                inline: Length::zero(),
                                block: ifc.current_nesting_level.max_block_size_of_fragments_so_far,
                            },
                            Display::None => panic!("abspos box cannot be display:none"),
                        };
                        ifc.absolutely_positioned_fragments
                            .push(box_.layout(initial_start_corner, tree_rank));
                    }
                }
            } else
            // Reached the end of ifc.remaining_boxes
            if let Some(partial) = ifc.partial_inline_boxes_stack.pop() {
                partial.finish_layout(&mut ifc)
            } else {
                return (
                    ifc.current_nesting_level.fragments_so_far,
                    ifc.absolutely_positioned_fragments,
                    ifc.current_nesting_level.max_block_size_of_fragments_so_far,
                );
            }
        }
    }
}

impl InlineBox {
    fn start_layout<'box_tree>(
        &'box_tree self,
        ifc: &mut InlineFormattingContextState<'box_tree, '_>,
    ) -> PartialInlineBoxFragment<'box_tree> {
        let style = self.style.clone();
        let cbis = ifc.containing_block.inline_size;
        let mut padding = style.padding().map(|v| v.percentage_relative_to(cbis));
        let mut border = style.border_width().map(|v| v.percentage_relative_to(cbis));
        let mut margin = style
            .margin()
            .map(|v| v.auto_is(Length::zero).percentage_relative_to(cbis));
        if self.first_fragment {
            ifc.inline_position += padding.inline_start + border.inline_start + margin.inline_start;
        } else {
            padding.inline_start = Length::zero();
            border.inline_start = Length::zero();
            margin.inline_start = Length::zero();
        }
        let content_rect = Rect {
            start_corner: Vec2 {
                block: padding.block_start + border.block_start + margin.block_start,
                inline: ifc.inline_position,
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
                &mut ifc.current_nesting_level,
                InlineNestingLevelState {
                    remaining_boxes: self.children.iter(),
                    fragments_so_far: Vec::with_capacity(self.children.len()),
                    max_block_size_of_fragments_so_far: Length::zero(),
                },
            ),
        }
    }
}

impl<'box_tree> PartialInlineBoxFragment<'box_tree> {
    fn finish_layout(mut self, ifc: &mut InlineFormattingContextState<'box_tree, '_>) {
        let mut fragment = self.fragment;
        fragment.content_rect.size = Vec2 {
            inline: ifc.inline_position - fragment.content_rect.start_corner.inline,
            block: ifc.current_nesting_level.max_block_size_of_fragments_so_far,
        };
        fragment.content_rect.start_corner += &relative_adjustement(
            &fragment.style,
            ifc.containing_block.inline_size,
            ifc.containing_block.block_size,
        );
        if self.last_fragment {
            ifc.inline_position += fragment.padding.inline_end
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
        fragment.children = ifc.current_nesting_level.fragments_so_far.take();
        debug_assert!(ifc
            .current_nesting_level
            .remaining_boxes
            .as_slice()
            .is_empty());
        ifc.current_nesting_level = self.saved_state;
        ifc.current_nesting_level
            .fragments_so_far
            .push(Fragment::Box(fragment));
    }
}

impl TextRun {
    fn layout(&self, ifc: &mut InlineFormattingContextState) {
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
                inline: ifc.inline_position,
            },
            size: Vec2 {
                block: line_height,
                inline: inline_size,
            },
        };
        ifc.inline_position += inline_size;
        ifc.current_nesting_level
            .max_block_size_of_fragments_so_far
            .max_assign(line_height);
        ifc.current_nesting_level
            .fragments_so_far
            .push(Fragment::Text(TextFragment {
                parent_style,
                content_rect,
                text,
            }));
    }
}
