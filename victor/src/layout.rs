mod boxes;
mod fragments;

use crate::geom::flow_relative::Vec2;
use crate::geom::Length;
use crate::style::values::{Direction, WritingMode};

impl crate::dom::Document {
    pub fn render(&self, viewport: crate::primitives::Size<crate::primitives::CssPx>) {
        let box_tree = self.box_tree();

        // FIXME: use the document’s mode:
        // https://drafts.csswg.org/css-writing-modes/#principal-flow
        let initial_containing_block = ContainingBlock {
            inline_size: Length { px: viewport.width },
            // block_size: Some(Length {
            //     px: viewport.height,
            // }),
            mode: (WritingMode::HorizontalTb, Direction::Ltr),
        };

        let _ = box_tree.layout(&initial_containing_block);
    }
}

struct ContainingBlock {
    inline_size: Length,
    // block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

impl boxes::BlockFormattingContext {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<fragments::Block>, Length) {
        self.0.layout(containing_block)
    }
}

impl boxes::BlockContainer {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<fragments::Block>, Length) {
        match self {
            boxes::BlockContainer::BlockLevels(child_boxes) => {
                let mut block_size = Length::zero();
                let mut child_fragments = Vec::new();
                for child in child_boxes {
                    let (fragment, margin_height) = child.layout(containing_block, block_size);
                    // FIXME: margin collapsing
                    block_size += margin_height;
                    child_fragments.push(fragment);
                }
                (child_fragments, block_size)
            }
            boxes::BlockContainer::InlineFormattingContext(_children) => unimplemented!(),
        }
    }
}

impl boxes::BlockLevel {
    fn layout(
        &self,
        containing_block: &ContainingBlock,
        block_size_before: Length,
    ) -> (fragments::Block, Length) {
        match self {
            boxes::BlockLevel::SameFormattingContextBlock { style, contents } => {
                let cbi = containing_block.inline_size;
                let zero = Length::zero();
                let padding = style.padding().map(|v| v.percentage_relative_to(cbi));
                let border = style.border_width().map(|v| v.percentage_relative_to(cbi));
                // FIXME: width and height properties, then auto margins
                let margin = style
                    .margin()
                    .map(|v| v.auto_is(zero).percentage_relative_to(cbi));
                let pbm = &(&padding + &border) + &margin;
                let mut content_start_corner = pbm.start_corner();
                content_start_corner.block += block_size_before;
                let containing_block_for_children = ContainingBlock {
                    inline_size: containing_block.inline_size - pbm.inline_sum(),
                    // block_size: None,
                    mode: style.writing_mode(),
                };
                assert_eq!(
                    containing_block.mode, containing_block_for_children.mode,
                    "Mixed writing modes are not supported yet"
                );
                let (children, content_block_size) =
                    contents.layout(&containing_block_for_children);
                let content_size = Vec2 {
                    block: content_block_size,
                    inline: containing_block_for_children.inline_size,
                };
                let block = fragments::Block {
                    style: style.clone(),
                    children,
                    content_start_corner,
                    content_size,
                    padding,
                    border,
                    margin,
                };
                let margin_height = pbm.block_sum() + content_block_size;
                (block, margin_height)
            }
        }
    }
}
