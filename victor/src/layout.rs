use crate::dom;
use crate::style::values::*;
use crate::style::*;
use html5ever::tendril::StrTendril;

type BoxTreeRoot = FormattingContext;

enum FormattingContext {
    // Not included: inline formatting context, which is always part of a block container
    Flow(BlockFormattingContext),
    // Replaced(ReplacedElement), // Not called FC in specs, but behaves close enough
    // Table(Table),
    // Other layout modes go here
}

struct BlockFormattingContext(BlockContainer);

enum BlockContainer {
    Blocks(Vec<BlockLevel>),
    InlineFormattingContext(Vec<InlineLevel>),
}

enum BlockLevel {
    SameFormattingContextBlock(BlockContainer),
    // Other(FormattingContext),
}

enum InlineLevel {
    Text(StrTendril),
    Inline(Vec<InlineLevel>),
    // Atomic(FormattingContext),
}

impl<'arena> dom::Document<'arena> {
    pub fn render(&self) {
        let mut builder = StyleSetBuilder::new();
        self.parse_stylesheets(&mut builder);
        let author_styles = builder.finish();

        let root_element = self.root_element();
        let root_element_style = cascade(&author_styles, root_element, None);
        // https://drafts.csswg.org/css-display-3/#transformations
        // The root element’s display type is always blockified.
        let _box_tree_root: BoxTreeRoot =
            blockify(&author_styles, root_element, &root_element_style);
    }
}

fn blockify(
    author_styles: &StyleSet,
    element: dom::NodeRef,
    style: &ComputedValues,
) -> FormattingContext {
    match style.display.display {
        Display::None => {
            FormattingContext::Flow(BlockFormattingContext(BlockContainer::Blocks(Vec::new())))
        }
        Display::Other {
            inside: DisplayInside::Flow,
            ..
        } => FormattingContext::Flow(BlockFormattingContext(BlockContainer::new(
            author_styles,
            element,
            style,
        ))),
    }
}

impl BlockContainer {
    fn new(author_styles: &StyleSet, element: dom::NodeRef, element_syle: &ComputedValues) -> Self {
        BlockContainerBuilder::from_child_elements(author_styles, element, element_syle).build()
    }
}

trait Builder {
    fn push_text(&mut self, text: &StrTendril);

    fn push_inline(&mut self, inline: InlineLevel);

    fn push_block(&mut self, block: BlockLevel);

    fn from_child_elements(
        author_styles: &StyleSet,
        element: dom::NodeRef,
        element_style: &ComputedValues,
    ) -> Self
    where
        Self: Default,
    {
        let mut builder = Self::default();
        let first_child = if let Some(first) = element.first_child.get() {
            first
        } else {
            return builder
        };
        for child in first_child.self_and_next_siblings() {
            match &child.data {
                dom::NodeData::Document
                | dom::NodeData::Doctype { .. }
                | dom::NodeData::Comment { .. }
                | dom::NodeData::ProcessingInstruction { .. } => continue,
                dom::NodeData::Text { contents } => {
                    builder.push_text(&contents.borrow());
                    continue
                }
                dom::NodeData::Element(_) => {}
            }
            let style = cascade(author_styles, child, Some(element_style));
            match style.display.display {
                Display::None => {}
                Display::Other {
                    outside: DisplayOutside::Inline,
                    inside: DisplayInside::Flow,
                } => {
                    let InlineBuilder {
                        self_fragments_split_by_blocks,
                        children: grand_children,
                    } = InlineBuilder::from_child_elements(author_styles, element, element_style);
                    for (previous_grand_children, block) in self_fragments_split_by_blocks {
                        if !previous_grand_children.is_empty() {
                            builder.push_inline(InlineLevel::Inline(previous_grand_children))
                        }
                        builder.push_block(block)
                    }
                    if !grand_children.is_empty() {
                        builder.push_inline(InlineLevel::Inline(grand_children))
                    }
                }
                Display::Other {
                    outside: DisplayOutside::Block,
                    inside: DisplayInside::Flow,
                } => builder.push_block(BlockLevel::SameFormattingContextBlock(
                    BlockContainer::new(author_styles, element, &style),
                )),
            }
        }
        builder
    }
}

#[derive(Default)]
struct BlockContainerBuilder {
    blocks: Vec<BlockLevel>,
    consecutive_inlines: Vec<InlineLevel>,
}

impl Builder for BlockContainerBuilder {
    fn push_text(&mut self, text: &StrTendril) {
        inline_level_push_text(&mut self.consecutive_inlines, text)
    }

    fn push_inline(&mut self, inline: InlineLevel) {
        self.consecutive_inlines.push(inline)
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

#[derive(Default)]
struct InlineBuilder {
    self_fragments_split_by_blocks: Vec<(Vec<InlineLevel>, BlockLevel)>,
    children: Vec<InlineLevel>,
}

impl Builder for InlineBuilder {
    fn push_text(&mut self, text: &StrTendril) {
        inline_level_push_text(&mut self.children, text)
    }

    fn push_inline(&mut self, inline: InlineLevel) {
        self.children.push(inline)
    }

    fn push_block(&mut self, block: BlockLevel) {
        self.self_fragments_split_by_blocks
            .push((self.children.take(), block))
    }
}

fn inline_level_push_text(inlines: &mut Vec<InlineLevel>, text: &StrTendril) {
    if let Some(InlineLevel::Text(last_text)) = inlines.last_mut() {
        last_text.push_tendril(text)
    } else {
        inlines.push(InlineLevel::Text(text.clone()))
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
