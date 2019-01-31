mod boxes;
mod fragments;

use crate::geom::flow_relative::Vec2;
use crate::geom::Length;
use crate::style::values::{Direction, LengthOrPercentageOrAuto, WritingMode};

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
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<fragments::Fragment>, Length) {
        self.0.layout(containing_block)
    }
}

impl boxes::BlockContainer {
    fn layout(&self, containing_block: &ContainingBlock) -> (Vec<fragments::Fragment>, Length) {
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
    ) -> (fragments::Fragment, Length) {
        match self {
            boxes::BlockLevel::SameFormattingContextBlock { style, contents } => {
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
                    margin = computed_margin.map_inline_and_block_axes(
                        |v| v.auto_is(|| unreachable!()),
                        |v| v.auto_is(|| zero),
                    );
                } else {
                    inline_size = None; // auto
                    margin = computed_margin.map(|v| v.auto_is(|| zero));
                }
                let margin = margin.map(|v| v.percentage_relative_to(cbis));
                let pbm = &pb + &margin;
                let inline_size = inline_size.unwrap_or_else(|| cbis - pbm.inline_sum());
                let mut content_start_corner = pbm.start_corner();
                content_start_corner.block += block_size_before;
                let containing_block_for_children = ContainingBlock {
                    inline_size,
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
                let block = fragments::Fragment {
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
