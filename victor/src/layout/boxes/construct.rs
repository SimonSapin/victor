use super::*;
use crate::dom;
use crate::layout::Take;
use crate::style::values::{Display, DisplayInside, DisplayOutside, Position};
use crate::style::{style_for_element, StyleSet, StyleSetBuilder};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

impl dom::Document {
    pub(in crate::layout) fn box_tree(&self) -> BoxTreeRoot {
        let mut builder = StyleSetBuilder::new();
        self.parse_stylesheets(&mut builder);
        let author_styles = builder.finish();

        let context = Context {
            document: self,
            author_styles: &author_styles,
        };

        BlockFormattingContext::build(&context, dom::Document::document_node_id(), None)
    }
}

/// The context.
///
/// Used by the block container builder.
struct Context<'a> {
    document: &'a dom::Document,
    author_styles: &'a StyleSet,
}

enum IntermediateBlockLevelBox {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: IntermediateBlockContainer,
    },
    OutOfFlowAbsolutelyPositionedBox {
        style: Arc<ComputedValues>,
        element: dom::NodeId,
    },
}

/// A block container that may still have to be constructed.
///
/// Represents either the inline formatting context of an anonymous block
/// box or the yet-to-be-computed block container generated from the children
/// of a given element.
///
/// Deferring allows using rayon’s `into_par_iter`.
enum IntermediateBlockContainer {
    InlineFormattingContext(InlineFormattingContext),
    Deferred { from_children_of: dom::NodeId },
}

/// A builder for a block container.
///
/// This builder starts from the first child of a given DOM node
/// and does a preorder traversal of all of its inclusive siblings.
struct BlockContainerBuilder<'a> {
    context: &'a Context<'a>,
    /// The first child of the DOM node whose block container we are building.
    ///
    /// In the rest of the comments, the DOM node whose block container we
    /// are building is called the container root.
    first_child: Option<dom::NodeId>,
    /// The style of the container root, if any.
    parent_style: Option<&'a Arc<ComputedValues>>,
    /// The list of block-level boxes of the final block container.
    ///
    /// Contains all the complete block level boxes we found traversing the tree
    /// so far, if this is empty at the end of the traversal and the ongoing
    /// inline formatting context is not empty, the block container establishes
    /// an inline formatting context (see end of `build`).
    ///
    /// DOM nodes which represent block-level boxes are immediately pushed
    /// to this list with their style without ever being traversed at this
    /// point, instead we just move to their next sibling. If the DOM node
    /// doesn't have a next sibling, we either reached the end of the container
    /// root or there are ongoing inline-level boxes
    /// (see `handle_block_level_element`).
    block_level_boxes: Vec<IntermediateBlockLevelBox>,
    /// The ongoing inline formatting context of the builder.
    ///
    /// Contains all the complete inline level boxes we found traversing the
    /// tree so far. If a block-level box is found during traversal,
    /// this inline formatting context is pushed as a block level box to
    /// the list of block-level boxes of the builder
    /// (see `end_ongoing_inline_formatting_context`).
    ongoing_inline_formatting_context: InlineFormattingContext,
    /// The ongoing inline-level box stack of the builder.
    ///
    /// Contains all the currently ongoing inline-level boxes we entered so far.
    /// The traversal is at all times as deep in the tree as this stack is,
    /// which is why the code doesn't need to keep track of the actual
    /// container root (see `handle_inline_level_element`).
    ///
    /// Whenever the end of a DOM element that represents an inline-level box is
    /// reached, the inline box at the top of this stack is complete and ready
    /// to be pushed to the children of the next last ongoing inline
    /// level box or the ongoing inline formatting context if the stack is
    /// now empty, which means we reached the end of a child of the actual
    /// container root (see `move_to_next_sibling`).
    ongoing_inline_level_box_stack: Vec<InlineBox>,
    /// The style of the anonymous block boxes pushed to the list of block-level
    /// boxes, if any (see `end_ongoing_inline_formatting_context`).
    anonymous_style: Option<Arc<ComputedValues>>,
}

impl BlockFormattingContext {
    fn build<'a>(
        context: &'a Context<'a>,
        node: dom::NodeId,
        parent_style: Option<&'a Arc<ComputedValues>>,
    ) -> Self {
        let contents = BlockContainerBuilder::build(context, node, parent_style);
        Self { contents }
    }
}

impl<'a> BlockContainerBuilder<'a> {
    fn build(
        context: &'a Context<'a>,
        node: dom::NodeId,
        parent_style: Option<&'a Arc<ComputedValues>>,
    ) -> BlockContainer {
        let mut builder = Self {
            context,
            first_child: context.document[node].first_child,
            parent_style,
            block_level_boxes: Default::default(),
            ongoing_inline_formatting_context: Default::default(),
            ongoing_inline_level_box_stack: Default::default(),
            anonymous_style: Default::default(),
        };

        let mut next_descendant = builder.first_child;
        while let Some(descendant) = next_descendant.take() {
            match &builder.context.document[descendant].data {
                dom::NodeData::Document
                | dom::NodeData::Doctype { .. }
                | dom::NodeData::Comment { .. }
                | dom::NodeData::ProcessingInstruction { .. } => {
                    next_descendant = builder.move_to_next_sibling(descendant);
                }
                dom::NodeData::Text { contents } => {
                    next_descendant = builder.handle_text(descendant, contents);
                }
                dom::NodeData::Element(_) => {
                    next_descendant = builder.handle_element(descendant);
                }
            }
        }

        while !builder.ongoing_inline_level_box_stack.is_empty() {
            builder.end_ongoing_inline_level_box();
        }

        if !builder
            .ongoing_inline_formatting_context
            .inline_level_boxes
            .is_empty()
        {
            if builder.block_level_boxes.is_empty() {
                return BlockContainer::InlineFormattingContext(
                    builder.ongoing_inline_formatting_context,
                );
            }
            builder.end_ongoing_inline_formatting_context();
        }

        BlockContainer::BlockLevelBoxes(
            builder
                .block_level_boxes
                .into_par_iter()
                .map(|block_level_box| block_level_box.finish(context))
                .collect(),
        )
    }

    fn handle_text(&mut self, descendant: dom::NodeId, input: &str) -> Option<dom::NodeId> {
        let (leading_whitespace, mut input) = self.handle_leading_whitespace(input);
        if leading_whitespace || !input.is_empty() {
            // This text node should be pushed either to the next ongoing
            // inline level box with the parent style of that inline level box
            // that will be ended, or directly to the ongoing inline formatting
            // context with the parent style of that builder.
            let (inlines, parent_style) = self.current_inline_level_boxes_and_parent_style();

            let mut new_text_run_contents;
            let output;
            if let Some(InlineLevelBox::TextRun(TextRun { text, .. })) = inlines.last_mut() {
                // Append to the existing text run
                new_text_run_contents = None;
                output = text;
            } else {
                new_text_run_contents = Some(String::new());
                output = new_text_run_contents.as_mut().unwrap();
            }

            if leading_whitespace {
                output.push(' ')
            }
            loop {
                if let Some(i) = input.bytes().position(|b| b.is_ascii_whitespace()) {
                    let (non_whitespace, rest) = input.split_at(i);
                    output.push_str(non_whitespace);
                    output.push(' ');
                    if let Some(i) = rest.bytes().position(|b| !b.is_ascii_whitespace()) {
                        input = &rest[i..];
                    } else {
                        break;
                    }
                } else {
                    output.push_str(input);
                    break;
                }
            }

            if let Some(text) = new_text_run_contents {
                let parent_style = parent_style
                    .expect("found a text node without a parent")
                    .clone();
                inlines.push(InlineLevelBox::TextRun(TextRun { parent_style, text }))
            }
        }

        // Let .build continue the traversal from the next sibling of
        // the text node.
        self.move_to_next_sibling(descendant)
    }

    /// Returns:
    ///
    /// * Whether this text run has preserved (non-collapsible) leading whitespace
    /// * The contents starting at the first non-whitespace character (or the empty string)
    fn handle_leading_whitespace<'text>(&mut self, text: &'text str) -> (bool, &'text str) {
        // FIXME: this is only an approximation of
        // https://drafts.csswg.org/css2/text.html#white-space-model
        if !text.starts_with(|c: char| c.is_ascii_whitespace()) {
            return (false, text);
        }
        let mut inline_level_boxes = self.current_inline_level_boxes().as_slice();
        let preserved = loop {
            match inline_level_boxes.split_last() {
                Some((InlineLevelBox::InlineBox(b), _)) => inline_level_boxes = &b.children,
                Some((InlineLevelBox::OutOfFlowAbsolutelyPositionedBox(_), before)) => {
                    inline_level_boxes = before
                }
                Some((InlineLevelBox::TextRun(r), _)) => break !r.text.ends_with(' '),
                // Some(InlineLevelBox::Atomic(_)) => break false,
                None => break false, // Paragraph start
            }
        };
        let text = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
        (preserved, text)
    }

    fn handle_element(&mut self, descendant: dom::NodeId) -> Option<dom::NodeId> {
        let parent_style = self.current_parent_style();
        let descendant_style = style_for_element(
            self.context.author_styles,
            self.context.document,
            descendant,
            parent_style.map(|style| &**style),
        );
        match (
            descendant_style.box_.display,
            descendant_style.box_.position,
        ) {
            (Display::None, _) => self.move_to_next_sibling(descendant),
            (_, Position::Absolute) => {
                self.handle_absolutely_positioned_element(descendant, descendant_style)
            }
            (
                Display::Other {
                    outside: DisplayOutside::Inline,
                    inside: DisplayInside::Flow,
                },
                _,
            ) => self.handle_inline_level_element(descendant, descendant_style),
            (
                Display::Other {
                    outside: DisplayOutside::Block,
                    inside: DisplayInside::Flow,
                },
                _,
            ) => self.handle_block_level_element(descendant, descendant_style),
        }
    }

    fn handle_inline_level_element(
        &mut self,
        descendant: dom::NodeId,
        descendant_style: Arc<ComputedValues>,
    ) -> Option<dom::NodeId> {
        // Whatever happened before, we just found an inline level element, so
        // all we need to do is to remember this ongoing inline level box.
        self.ongoing_inline_level_box_stack.push(InlineBox {
            style: descendant_style,
            first_fragment: true,
            last_fragment: false,
            children: vec![],
        });

        if let Some(first_child) = self.context.document[descendant].first_child {
            // This inline level element has children, let .build continue
            // the traversal from there.
            return Some(first_child);
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
                let fragmented = InlineBox {
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
            let mut fragmented_inline_level = InlineLevelBox::InlineBox(last);
            for mut fragmented_parent_inline_level_box in fragmented_inline_level_boxes {
                fragmented_parent_inline_level_box
                    .children
                    .push(fragmented_inline_level);
                fragmented_inline_level =
                    InlineLevelBox::InlineBox(fragmented_parent_inline_level_box);
            }

            self.ongoing_inline_formatting_context
                .inline_level_boxes
                .push(fragmented_inline_level);
        }

        // We found a block level element, so the ongoing inline formatting
        // context needs to be ended.
        self.end_ongoing_inline_formatting_context();

        self.block_level_boxes
            .push(IntermediateBlockLevelBox::SameFormattingContextBlock {
                style: descendant_style,
                contents: IntermediateBlockContainer::Deferred {
                    from_children_of: descendant,
                },
            });

        self.move_to_next_sibling(descendant)
    }

    fn handle_absolutely_positioned_element(
        &mut self,
        descendant: dom::NodeId,
        descendant_style: Arc<ComputedValues>,
    ) -> Option<dom::NodeId> {
        if self
            .ongoing_inline_formatting_context
            .inline_level_boxes
            .is_empty()
            && self.ongoing_inline_level_box_stack.is_empty()
        {
            self.block_level_boxes.push(
                IntermediateBlockLevelBox::OutOfFlowAbsolutelyPositionedBox {
                    style: descendant_style,
                    element: descendant,
                },
            )
        } else {
            let box_ = InlineLevelBox::OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox {
                contents: BlockFormattingContext::build(
                    self.context,
                    descendant,
                    Some(&descendant_style),
                ),
                style: descendant_style,
            });
            self.current_inline_level_boxes().push(box_)
        }
        self.move_to_next_sibling(descendant)
    }

    fn move_to_next_sibling(&mut self, descendant: dom::NodeId) -> Option<dom::NodeId> {
        let mut descendant_node = &self.context.document[descendant];
        if let Some(next_sibling) = descendant_node.next_sibling {
            // This descendant has a next sibling, just let .build continue
            // the traversal from there.
            return Some(next_sibling);
        }

        // This descendant has no next sibling, so it was the last child of its
        // parent, we go up the stack of ongoing inline level boxes, ending them
        // until we find one with a next sibling to let .build continue.
        while !self.ongoing_inline_level_box_stack.is_empty() {
            self.end_ongoing_inline_level_box();

            descendant_node = &self.context.document[descendant_node
                .parent
                .expect("found a descendant without a parent")];
            if let Some(next_sibling) = descendant_node.next_sibling {
                return Some(next_sibling);
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
            return;
        }

        let parent_style = self.parent_style.map(|s| &**s);
        let anonymous_style = self.anonymous_style.get_or_insert_with(|| {
            // If parent_style is None, the parent is the document node,
            // in which case anonymous inline boxes should inherit their
            // styles from initial values.
            ComputedValues::anonymous_inheriting_from(parent_style)
        });

        self.block_level_boxes
            .push(IntermediateBlockLevelBox::SameFormattingContextBlock {
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
            .expect("no ongoing inline level box found");
        last_ongoing_inline_level_box.last_fragment = true;
        self.current_inline_level_boxes()
            .push(InlineLevelBox::InlineBox(last_ongoing_inline_level_box));
    }

    fn current_inline_level_boxes_and_parent_style(
        &mut self,
    ) -> (&mut Vec<InlineLevelBox>, Option<&Arc<ComputedValues>>) {
        match self.ongoing_inline_level_box_stack.last_mut() {
            Some(last) => (&mut last.children, Some(&last.style)),
            None => (
                &mut self.ongoing_inline_formatting_context.inline_level_boxes,
                self.parent_style,
            ),
        }
    }

    fn current_inline_level_boxes(&mut self) -> &mut Vec<InlineLevelBox> {
        match self.ongoing_inline_level_box_stack.last_mut() {
            Some(last) => &mut last.children,
            None => &mut self.ongoing_inline_formatting_context.inline_level_boxes,
        }
    }

    fn current_parent_style(&self) -> Option<&Arc<ComputedValues>> {
        self.ongoing_inline_level_box_stack
            .last()
            .map(|last| &last.style)
            .or(self.parent_style)
    }
}

impl IntermediateBlockLevelBox {
    fn finish(self, context: &Context) -> BlockLevelBox {
        match self {
            IntermediateBlockLevelBox::SameFormattingContextBlock { style, contents } => {
                BlockLevelBox::SameFormattingContextBlock {
                    contents: contents.finish(context, &style),
                    style,
                }
            }
            IntermediateBlockLevelBox::OutOfFlowAbsolutelyPositionedBox { style, element } => {
                BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox {
                    contents: BlockFormattingContext::build(context, element, Some(&style)),
                    style: style,
                })
            }
        }
    }
}

impl IntermediateBlockContainer {
    fn finish(self, context: &Context, style: &Arc<ComputedValues>) -> BlockContainer {
        match self {
            IntermediateBlockContainer::Deferred { from_children_of } => {
                BlockContainerBuilder::build(context, from_children_of, Some(style))
            }
            IntermediateBlockContainer::InlineFormattingContext(ifc) => {
                BlockContainer::InlineFormattingContext(ifc)
            }
        }
    }
}
