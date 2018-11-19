use super::*;
use crate::dom;
use crate::style::values::*;
use crate::style::*;

impl<'arena> dom::Document<'arena> {
    pub fn render(&self) {
        let _ = self.box_tree();
    }

    fn box_tree(&self) -> BoxTreeRoot {
        let mut builder = StyleSetBuilder::new();
        self.parse_stylesheets(&mut builder);
        let author_styles = builder.finish();

        let root_element = self.root_element();
        let root_element_style = cascade(&author_styles, root_element, None);
        let mut builder = BlockContainerBuilder::default();
        builder.push_element(&author_styles, root_element, &root_element_style);
        FormattingContext::Flow(BlockFormattingContext(builder.build()))
    }
}

trait Builder {
    fn inlines(&mut self) -> &mut Vec<InlineLevel>;

    fn push_block(&mut self, block: BlockLevel);

    fn from_child_elements(
        author_styles: &StyleSet,
        parent_element: dom::NodeRef,
        parent_element_style: &ComputedValues,
    ) -> Self
    where
        Self: Default,
    {
        let mut builder = Self::default();
        if let Some(first_child) = parent_element.first_child.get() {
            for child in first_child.self_and_next_siblings() {
                match &child.data {
                    dom::NodeData::Document
                    | dom::NodeData::Doctype { .. }
                    | dom::NodeData::Comment { .. }
                    | dom::NodeData::ProcessingInstruction { .. } => continue,
                    dom::NodeData::Text { contents } => {
                        let text = contents.borrow();
                        let inlines = builder.inlines();
                        if let Some(InlineLevel::Text(last_text)) = inlines.last_mut() {
                            last_text.push_tendril(&text)
                        } else {
                            inlines.push(InlineLevel::Text(text.clone()))
                        }
                        continue
                    }
                    dom::NodeData::Element(_) => {
                        let style = cascade(author_styles, child, Some(parent_element_style));
                        builder.push_element(author_styles, child, &style)
                    }
                }
            }
        }
        builder
    }

    fn push_element(
        &mut self,
        author_styles: &StyleSet,
        element: dom::NodeRef,
        style: &ComputedValues,
    ) {
        match style.display.display {
            Display::None => {}
            Display::Other {
                outside: DisplayOutside::Inline,
                inside: DisplayInside::Flow,
            } => {
                let InlineBuilder {
                    self_fragments_split_by_blocks,
                    children: grand_children,
                } = InlineBuilder::from_child_elements(author_styles, element, style);
                for (previous_grand_children, block) in self_fragments_split_by_blocks {
                    if !previous_grand_children.is_empty() {
                        self.inlines()
                            .push(InlineLevel::Inline(previous_grand_children))
                    }
                    self.push_block(block)
                }
                if !grand_children.is_empty() {
                    self.inlines().push(InlineLevel::Inline(grand_children))
                }
            }
            Display::Other {
                outside: DisplayOutside::Block,
                inside: DisplayInside::Flow,
            } => self.push_block(BlockLevel::SameFormattingContextBlock(
                BlockContainerBuilder::from_child_elements(author_styles, element, &style).build(),
            )),
        }
    }
}

#[derive(Default)]
struct InlineBuilder {
    self_fragments_split_by_blocks: Vec<(Vec<InlineLevel>, BlockLevel)>,
    children: Vec<InlineLevel>,
}

impl Builder for InlineBuilder {
    fn inlines(&mut self) -> &mut Vec<InlineLevel> {
        &mut self.children
    }

    fn push_block(&mut self, block: BlockLevel) {
        self.self_fragments_split_by_blocks
            .push((self.children.take(), block))
    }
}

#[derive(Default)]
struct BlockContainerBuilder {
    blocks: Vec<BlockLevel>,
    consecutive_inlines: Vec<InlineLevel>,
}

impl Builder for BlockContainerBuilder {
    fn inlines(&mut self) -> &mut Vec<InlineLevel> {
        &mut self.consecutive_inlines
    }

    fn push_block(&mut self, block: BlockLevel) {
        if !self.consecutive_inlines.is_empty() {
            self.wrap_inlines_in_anonymous_block();
        }
        self.blocks.push(block)
    }
}
impl BlockContainerBuilder {
    fn wrap_inlines_in_anonymous_block(&mut self) {
        self.blocks.push(BlockLevel::SameFormattingContextBlock(
            BlockContainer::InlineFormattingContext(self.consecutive_inlines.take()),
        ));
    }

    fn build(mut self) -> BlockContainer {
        if !self.consecutive_inlines.is_empty() {
            if self.blocks.is_empty() {
                return BlockContainer::InlineFormattingContext(self.consecutive_inlines)
            }
            self.wrap_inlines_in_anonymous_block()
        }
        BlockContainer::Blocks(self.blocks)
    }
}

trait Take {
    fn take(&mut self) -> Self;
}

impl<T> Take for Vec<T> {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Vec::new())
    }
}
