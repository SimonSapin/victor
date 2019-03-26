use super::*;
use crate::dom;
use crate::fonts::BITSTREAM_VERA_SANS;
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

        IntermediateBlockFormattingContextBuilder::new_for_root(&context)
            .build()
            .finish(&context)
    }
}

struct Context<'a> {
    document: &'a dom::Document,
    author_styles: &'a StyleSet,
}

enum IntermediateBlockFormattingContext {
    InlineFormattingContext(IntermediateInlineFormattingContext),
    BlockLevels(Vec<IntermediateBlockLevel>),
}

enum IntermediateBlockLevel {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: IntermediateBlockContainer,
    },
}

enum IntermediateBlockContainer {
    Block(dom::NodeId),
    InlineFormattingContext(IntermediateInlineFormattingContext),
}

#[derive(Default)]
struct IntermediateInlineFormattingContext {
    inline_level_boxes: Vec<InlineLevel>,
    text: Vec<IntermediateText>,
}

struct IntermediateText {
    parent_style: Arc<ComputedValues>,
    node: dom::NodeId,
}

struct IntermediateBlockFormattingContextBuilder<'a> {
    context: &'a Context<'a>,
    first_child: Option<dom::NodeId>,
    parent_style: Option<Arc<ComputedValues>>,
    block_level_boxes: Vec<IntermediateBlockLevel>,
    ongoing_inline_formatting_context: IntermediateInlineFormattingContext,
    ongoing_inline_level_box_stack: Vec<InlineLevelBox>,
    anonymous_style: Option<Arc<ComputedValues>>,
}

#[allow(dead_code)]
impl<'a> IntermediateBlockFormattingContextBuilder<'a> {
    fn new_for_root(context: &'a Context<'a>) -> Self {
        Self {
            context,
            first_child: context.document[dom::Document::document_node_id()].first_child,
            parent_style: None,
            block_level_boxes: Default::default(),
            ongoing_inline_formatting_context: Default::default(),
            ongoing_inline_level_box_stack: Default::default(),
            // If any, anonymous blocks wrapping inlines at the root level get
            // initial styles, they don’t have a parent element to inherit from.
            anonymous_style: Some(ComputedValues::initial()),
        }
    }

    fn new_for_descendant(
        context: &'a Context<'a>,
        element: dom::NodeId,
        style: Arc<ComputedValues>,
    ) -> Self {
        Self {
            context,
            first_child: context.document[element].first_child,
            parent_style: Some(style),
            block_level_boxes: Default::default(),
            ongoing_inline_formatting_context: Default::default(),
            ongoing_inline_level_box_stack: Default::default(),
            anonymous_style: Default::default(),
        }
    }

    fn build(&mut self) -> IntermediateBlockFormattingContext {
        let mut next_descendant = self.first_child;
        while let Some(descendant) = next_descendant.take() {
            match &self.context.document[descendant].data {
                dom::NodeData::Document
                | dom::NodeData::Doctype { .. }
                | dom::NodeData::Comment { .. }
                | dom::NodeData::ProcessingInstruction { .. } => {
                    next_descendant = self.move_to_next_sibling(descendant);
                }
                dom::NodeData::Text { contents } => {
                    next_descendant = self.handle_text(descendant, contents);
                }
                dom::NodeData::Element(_) => {
                    next_descendant = self.handle_element(descendant);
                }
            }
        }

        while !self.ongoing_inline_level_box_stack.is_empty() {
            self.end_ongoing_inline_level_box();
        }

        if !self
            .ongoing_inline_formatting_context
            .inline_level_boxes
            .is_empty()
        {
            if self.block_level_boxes.is_empty() {
                return IntermediateBlockFormattingContext::InlineFormattingContext(
                    self.ongoing_inline_formatting_context.take(),
                )
            }
            self.end_ongoing_inline_formatting_context();
        }

        IntermediateBlockFormattingContext::BlockLevels(self.block_level_boxes.take())
    }

    fn handle_text(&mut self, descendant: dom::NodeId, contents: &str) -> Option<dom::NodeId> {
        // FIXME: implement https://drafts.csswg.org/css2/text.html#white-space-model
        if !contents.as_bytes().iter().all(u8::is_ascii_whitespace) {
            let text_id = TextId(self.ongoing_inline_formatting_context.text.len());

            // This text node should be pushed either to the next ongoing
            // inline level box with the parent style of that inline level box
            // that will be ended, or directly to the ongoing inline formatting
            // context with the parent style of that builder.
            let (parent_style, inline_level_boxes) =
                self.ongoing_inline_level_box_stack.last_mut().map_or(
                    (
                        self.parent_style
                            .as_ref()
                            .expect("text node in a document always has a parent"),
                        &mut self.ongoing_inline_formatting_context.inline_level_boxes,
                    ),
                    |last| (&last.style, &mut last.children),
                );
            self.ongoing_inline_formatting_context
                .text
                .push(IntermediateText {
                    parent_style: parent_style.clone(),
                    node: descendant,
                });
            inline_level_boxes.push(InlineLevel::Text(text_id));
        }

        // Let .build continue the traversal from the next sibling of
        // the text node.
        self.move_to_next_sibling(descendant)
    }

    fn handle_element(&mut self, descendant: dom::NodeId) -> Option<dom::NodeId> {
        let parent_style = self
            .ongoing_inline_level_box_stack
            .last()
            .map_or(self.parent_style.as_ref().map(|style| &**style), |last| Some(&last.style));
        let descendant_style = style_for_element(
            self.context.author_styles,
            self.context.document,
            descendant,
            parent_style,
        );
        match descendant_style.display.display {
            Display::None => self.move_to_next_sibling(descendant),
            Display::Other {
                outside: DisplayOutside::Inline,
                inside: DisplayInside::Flow,
            } => self.handle_inline_level_element(descendant, descendant_style),
            Display::Other {
                outside: DisplayOutside::Block,
                inside: DisplayInside::Flow,
            } => self.handle_block_level_element(descendant, descendant_style),
        }
    }

    fn handle_inline_level_element(
        &mut self,
        descendant: dom::NodeId,
        descendant_style: Arc<ComputedValues>,
    ) -> Option<dom::NodeId> {
        // Whatever happened before, we just found an inline level element, so
        // all we need to do is to remember this ongoing inline level box.
        self.ongoing_inline_level_box_stack.push(InlineLevelBox {
            style: descendant_style,
            first_fragment: true,
            last_fragment: false,
            children: vec![],
        });

        if let Some(first_child) = self.context.document[descendant].first_child {
            // This inline level element has children, let .build continue
            // the traversal from there.
            return Some(first_child)
        }

        // This inline level element didn't have any children, so we end
        // the ongoing inline level box we just pushed.
        self.end_ongoing_inline_level_box();

        // Let .build continue the traversal from the next sibling of
        // the element.
        self.move_to_next_sibling(descendant)
    }

    fn handle_block_level_element(
        &mut self,
        descendant: dom::NodeId,
        descendant_style: Arc<ComputedValues>,
    ) -> Option<dom::NodeId> {
        // We just found a block level element, all ongoing inline level boxes
        // need to be split around it. We iterate on the fragmented inline
        // level box stack to take their contents and set their first_fragment
        // field to false, for the fragmented inline level boxes that will
        // come after the block level element.
        let mut fragmented_inline_level_boxes = self
            .ongoing_inline_level_box_stack
            .iter_mut()
            .rev()
            .map(|ongoing| {
                let fragmented = InlineLevelBox {
                    style: ongoing.style.clone(),
                    first_fragment: ongoing.first_fragment,
                    // The fragmented boxes before the block level element
                    // are obviously not the last fragment.
                    last_fragment: false,
                    children: ongoing.children.take(),
                };
                ongoing.first_fragment = false;
                fragmented
            });

        if let Some(last) = fragmented_inline_level_boxes.next() {
            // There were indeed some ongoing inline level boxes before
            // the block, we accumulate them as a single inline level box
            // to be pushed to the ongoing inline formatting context.

            let mut fragmented_inline_level = InlineLevel::Box(last);
            for mut fragmented_parent_inline_level_box in fragmented_inline_level_boxes {
                fragmented_parent_inline_level_box
                    .children
                    .push(fragmented_inline_level);
                fragmented_inline_level = InlineLevel::Box(fragmented_parent_inline_level_box);
            }

            self.ongoing_inline_formatting_context
                .inline_level_boxes
                .push(fragmented_inline_level);
        }

        // We found a block level element, so the ongoing inline formatting
        // context needs to be ended.
        self.end_ongoing_inline_formatting_context();

        self.block_level_boxes
            .push(IntermediateBlockLevel::SameFormattingContextBlock {
                style: descendant_style,
                contents: IntermediateBlockContainer::Block(descendant),
            });

        self.move_to_next_sibling(descendant)
    }

    fn move_to_next_sibling(&mut self, descendant: dom::NodeId) -> Option<dom::NodeId> {
        let mut descendant_node = &self.context.document[descendant];
        if let Some(next_sibling) = descendant_node.next_sibling {
            // This descendant has a next sibling, just let .build continue
            // the traversal from there.
            return Some(next_sibling)
        }

        // This descendant has no next sibling, so it was the last child of its
        // parent, we go up the stack of ongoing inline level boxes, ending them
        // until we find one with a next sibling to let .build continue.
        while !self.ongoing_inline_level_box_stack.is_empty() {
            self.end_ongoing_inline_level_box();

            descendant_node =
                &self.context.document[descendant_node.parent.expect("descendant has a parent")];
            if let Some(next_sibling) = descendant_node.next_sibling {
                return Some(next_sibling)
            }
        }

        // There are no ongoing inline level boxes anymore, this descendant is
        // the last child of the root of this builder, the traversal will stop.
        None
    }

    fn end_ongoing_inline_formatting_context(&mut self) {
        assert!(
            self.ongoing_inline_level_box_stack.is_empty(),
            "there should be no ongoing inline level boxes",
        );

        if self
            .ongoing_inline_formatting_context
            .inline_level_boxes
            .is_empty()
        {
            // There should never be an empty inline formatting context.
            return
        }

        let parent_style = self.parent_style.as_ref();
        let anonymous_style = self.anonymous_style.get_or_insert_with(|| {
            ComputedValues::anonymous_inheriting_from(
                parent_style.expect("anonymous style is never None when parent style is"),
            )
        });

        self.block_level_boxes
            .push(IntermediateBlockLevel::SameFormattingContextBlock {
                style: anonymous_style.clone(),
                contents: IntermediateBlockContainer::InlineFormattingContext(
                    self.ongoing_inline_formatting_context.take(),
                ),
            });
    }

    fn end_ongoing_inline_level_box(&mut self) {
        let mut last_ongoing_inline_level_box = self
            .ongoing_inline_level_box_stack
            .pop()
            .expect("there is an ongoing inline level box");

        last_ongoing_inline_level_box.last_fragment = true;

        // The inline level box we just ended should be either pushed to the
        // next ongoing inline level box that will be ended or directly to
        // the ongoing inline formatting context.
        let inline_level_boxes = self.ongoing_inline_level_box_stack.last_mut().map_or(
            &mut self.ongoing_inline_formatting_context.inline_level_boxes,
            |last| &mut last.children,
        );

        inline_level_boxes.push(InlineLevel::Box(last_ongoing_inline_level_box));
    }
}

impl IntermediateBlockFormattingContext {
    fn finish(self, context: &Context) -> BlockFormattingContext {
        BlockFormattingContext(match self {
            IntermediateBlockFormattingContext::InlineFormattingContext(
                intermediate_inline_formatting_context,
            ) => BlockContainer::InlineFormattingContext(
                intermediate_inline_formatting_context.finish(context),
            ),
            IntermediateBlockFormattingContext::BlockLevels(intermediate_block_levels) => {
                BlockContainer::BlockLevels(
                    intermediate_block_levels
                        .into_iter()
                        .map(|block_level| block_level.finish(context))
                        .collect(),
                )
            }
        })
    }
}

impl IntermediateBlockLevel {
    fn finish(self, context: &Context) -> BlockLevel {
        match self {
            IntermediateBlockLevel::SameFormattingContextBlock { style, contents } => {
                BlockLevel::SameFormattingContextBlock {
                    contents: contents.finish(context, &style),
                    style,
                }
            }
        }
    }
}

impl IntermediateBlockContainer {
    fn finish(self, context: &Context, style: &Arc<ComputedValues>) -> BlockContainer {
        match self {
            IntermediateBlockContainer::Block(block) => {
                IntermediateBlockFormattingContextBuilder::new_for_descendant(
                    context,
                    block,
                    style.clone(),
                )
                .build()
                .finish(context)
                .0
            }
            IntermediateBlockContainer::InlineFormattingContext(
                intermediate_inline_formatting_context,
            ) => BlockContainer::InlineFormattingContext(
                intermediate_inline_formatting_context.finish(context),
            ),
        }
    }
}

impl IntermediateInlineFormattingContext {
    fn finish(self, context: &Context) -> InlineFormattingContext {
        InlineFormattingContext {
            inline_level_boxes: self.inline_level_boxes,
            text: self
                .text
                .into_iter()
                .map(|text| text.finish(context))
                .collect(),
        }
    }
}

impl IntermediateText {
    fn finish(self, context: &Context) -> Text {
        let contents = match &context.document[self.node].data {
            dom::NodeData::Text { contents } => contents,
            _ => panic!("node should be a text node"),
        };

        let mut segment = ShapedSegment::new_with_naive_shaping(BITSTREAM_VERA_SANS.clone());
        segment.append(contents.chars()).unwrap();

        Text {
            parent_style: self.parent_style,
            segment,
        }
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

impl Take for IntermediateInlineFormattingContext {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Default::default())
    }
}
