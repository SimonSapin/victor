use super::*;
use crate::dom;
use crate::style::values::{Display, DisplayInside, DisplayOutside};
use crate::style::{style_for_element, StyleSet, StyleSetBuilder};

impl dom::Document {
    pub(in crate::layout) fn box_tree(&self) -> BoxTreeRoot {
        let mut builder = StyleSetBuilder::new();
        self.parse_stylesheets(&mut builder);
        let author_styles = builder.finish();
        let context = Context {
            document: self,
            author_styles: &author_styles,
        };

        let root_element = self.root_element();
        let root_element_style = style_for_element(&author_styles, self, root_element, None);
        // If any, anonymous blocks wrapping inlines at the root level get initial styles,
        // they don’t have a parent element to inherit from.
        let initial_values = ComputedValues::initial();
        let mut builder = Builder::<BlockContainerBuilderExtra>::new(initial_values);
        builder.push_element(&context, root_element, root_element_style);
        let (_, block) = builder.build();
        BlockFormattingContext(block)
    }
}

struct Context<'a> {
    document: &'a dom::Document,
    author_styles: &'a StyleSet,
}

struct Builder<Extra> {
    style: Arc<ComputedValues>,
    consecutive_inline_levels: Vec<InlineLevel>,
    extra: Extra,
}

impl<Extra: Default + PushBlock> Builder<Extra> {
    fn new(style: Arc<ComputedValues>) -> Self {
        Self {
            style,
            consecutive_inline_levels: Vec::new(),
            extra: Extra::default(),
        }
    }

    fn push_child_elements(&mut self, context: &Context, parent_element: dom::NodeId) {
        if let Some(first_child) = context.document[parent_element].first_child {
            for child in context.document.node_and_next_siblings(first_child) {
                match &context.document[child].data {
                    dom::NodeData::Document
                    | dom::NodeData::Doctype { .. }
                    | dom::NodeData::Comment { .. }
                    | dom::NodeData::ProcessingInstruction { .. } => {}
                    dom::NodeData::Text { contents } => self.push_text(contents),
                    dom::NodeData::Element(_) => {
                        let style = style_for_element(
                            context.author_styles,
                            context.document,
                            child,
                            Some(&self.style),
                        );
                        self.push_element(context, child, style)
                    }
                }
            }
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some(InlineLevel::Text(last_text)) = self.consecutive_inline_levels.last_mut() {
            last_text.push_str(&text)
        } else {
            self.consecutive_inline_levels
                .push(InlineLevel::Text(text.to_owned()))
        }
    }

    fn push_element(&mut self, context: &Context, element: dom::NodeId, style: Arc<ComputedValues>) {
        match style.display.display {
            Display::None => {}
            Display::Other {
                outside: DisplayOutside::Inline,
                inside: DisplayInside::Flow,
            } => {
                let mut builder = Builder::<InlineBuilderExtra>::new(style);
                builder.push_child_elements(context, element);
                let mut first = true;
                for (previous_grand_children, block) in
                    builder.extra.self_fragments_split_by_block_levels
                {
                    self.consecutive_inline_levels.push(InlineLevel::Inline {
                        style: Arc::clone(&builder.style),
                        first_fragment: first,
                        last_fragment: false,
                        children: previous_grand_children,
                    });
                    first = false;
                    // FIXME: wrap this block in an anonymous block that inherits
                    // **some** properties from the inline being split,
                    // in order to handle cases like this inline being `position: relative`.
                    // https://github.com/servo/servo/issues/22397#issuecomment-446678506
                    Extra::push_block(self, block)
                }
                let grand_children = builder.consecutive_inline_levels;
                self.consecutive_inline_levels.push(InlineLevel::Inline {
                    style: builder.style,
                    first_fragment: first,
                    last_fragment: true,
                    children: grand_children,
                })
            }
            Display::Other {
                outside: DisplayOutside::Block,
                inside: DisplayInside::Flow,
            } => {
                let mut builder = Builder::<BlockContainerBuilderExtra>::new(style);
                builder.push_child_elements(context, element);
                let (style, contents) = builder.build();
                Extra::push_block(
                    self,
                    BlockLevel::SameFormattingContextBlock { style, contents },
                )
            }
        }
    }
}

trait PushBlock: Sized {
    fn push_block(builder: &mut Builder<Self>, block: BlockLevel);
}

#[derive(Default)]
struct InlineBuilderExtra {
    self_fragments_split_by_block_levels: Vec<(Vec<InlineLevel>, BlockLevel)>,
}

impl PushBlock for InlineBuilderExtra {
    fn push_block(builder: &mut Builder<Self>, block: BlockLevel) {
        builder
            .extra
            .self_fragments_split_by_block_levels
            .push((builder.consecutive_inline_levels.take(), block))
    }
}

#[derive(Default)]
struct BlockContainerBuilderExtra {
    block_levels: Vec<BlockLevel>,
}

impl PushBlock for BlockContainerBuilderExtra {
    fn push_block(builder: &mut Builder<Self>, block: BlockLevel) {
        if !builder.consecutive_inline_levels.is_empty() {
            builder.wrap_inlines_in_anonymous_block();
        }
        builder.extra.block_levels.push(block)
    }
}
impl Builder<BlockContainerBuilderExtra> {
    fn wrap_inlines_in_anonymous_block(&mut self) {
        self.extra
            .block_levels
            .push(BlockLevel::SameFormattingContextBlock {
                style: ComputedValues::anonymous_inheriting_from(&self.style),
                contents: BlockContainer::InlineFormattingContext(
                    self.consecutive_inline_levels.take(),
                ),
            });
    }

    fn build(mut self) -> (Arc<ComputedValues>, BlockContainer) {
        if !self.consecutive_inline_levels.is_empty() {
            if self.extra.block_levels.is_empty() {
                return (
                    self.style,
                    BlockContainer::InlineFormattingContext(self.consecutive_inline_levels),
                )
            }
            self.wrap_inlines_in_anonymous_block()
        }
        (
            self.style,
            BlockContainer::BlockLevels(self.extra.block_levels),
        )
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
