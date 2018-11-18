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
    InlineFormattingContext { children: Vec<InlineLevel> },
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
        let root_element_style = cascade(root_element, &author_styles, None);
        // https://drafts.csswg.org/css-display-3/#transformations
        // The root element’s display type is always blockified.
        let _box_tree_root: BoxTreeRoot =
            blockify(root_element, &author_styles, &root_element_style);
    }
}

fn blockify(
    element: dom::NodeRef,
    author_styles: &StyleSet,
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
            element.first_child.get(),
            author_styles,
            style,
        ))),
    }
}

impl BlockContainer {
    fn new(
        first_child: Option<dom::NodeRef>,
        author_styles: &StyleSet,
        parent_style: &ComputedValues,
    ) -> Self {
        let mut builder = BlockContainerBuilder::default();
        let first_child = if let Some(first) = first_child {
            first
        } else {
            return BlockContainer::Blocks(Vec::new())
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
            let style = cascade(child, author_styles, Some(parent_style));
            match style.display.display {
                Display::None => {}
                Display::Other {
                    outside: DisplayOutside::Inline,
                    inside,
                } => builder.push_inline(InlineLevel::new(child, author_styles, &style, inside)),
                Display::Other {
                    outside: DisplayOutside::Block,
                    inside,
                } => builder.push_block(BlockLevel::new(child, author_styles, &style, inside)),
            }
        }
        builder.build()
    }
}

#[derive(Default)]
struct BlockContainerBuilder {
    blocks: Vec<BlockLevel>,
    consecutive_inlines: Vec<InlineLevel>,
}

impl BlockContainerBuilder {
    fn push_text(&mut self, text: &StrTendril) {
        if let Some(InlineLevel::Text(last_text)) = self.consecutive_inlines.last_mut() {
            last_text.push_tendril(text)
        } else {
            self.consecutive_inlines
                .push(InlineLevel::Text(text.clone()))
        }
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

    fn wrap_inlines_in_anonymous_block(&mut self) {
        self.blocks.push(BlockLevel::SameFormattingContextBlock(
            BlockContainer::InlineFormattingContext {
                children: std::mem::replace(&mut self.consecutive_inlines, Vec::new()),
            },
        ));
    }

    fn build(mut self) -> BlockContainer {
        if !self.consecutive_inlines.is_empty() {
            if self.blocks.is_empty() {
                return BlockContainer::InlineFormattingContext {
                    children: self.consecutive_inlines,
                }
            }
            self.wrap_inlines_in_anonymous_block()
        }
        BlockContainer::Blocks(self.blocks)
    }
}

impl BlockLevel {
    fn new(
        element: dom::NodeRef,
        author_styles: &StyleSet,
        style: &ComputedValues,
        inside: DisplayInside,
    ) -> Self {
        match inside {
            DisplayInside::Flow => BlockLevel::SameFormattingContextBlock(BlockContainer::new(
                element.first_child.get(),
                author_styles,
                style,
            )),
        }
    }
}

impl InlineLevel {
    fn new(
        _element: dom::NodeRef,
        _author_styles: &StyleSet,
        _style: &ComputedValues,
        inside: DisplayInside,
    ) -> Self {
        match inside {
            DisplayInside::Flow => {
                InlineLevel::Inline(Vec::new());
                unimplemented!()
            }
        }
    }
}
