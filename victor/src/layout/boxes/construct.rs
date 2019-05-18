use super::*;
use crate::dom;
use crate::dom::traversal::{Contents, Context, NonReplacedContents};
use crate::layout::Take;
use crate::style::values::{Display, DisplayGeneratingBox, DisplayInside, DisplayOutside};
use crate::style::{style_for_element, StyleSetBuilder};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon_croissant::ParallelIteratorExt;
use std::convert::TryInto;
use std::ops::BitOrAssign;

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
        let style = style_for_element(context.author_styles, context.document, root_element, None);

        let mut boxes = Vec::new();
        let contains_floats = build_for_root_element(&context, root_element, style, &mut boxes);
        BlockFormattingContext {
            contains_floats: contains_floats == ContainsFloats::Yes,
            contents: BlockContainer::BlockLevelBoxes(boxes),
        }
    }
}

fn build_for_root_element(
    context: &Context,
    root_element: dom::NodeId,
    style: Arc<ComputedValues>,
    boxes: &mut Vec<BlockLevelBox>,
) -> ContainsFloats {
    let replaced = ReplacedContent::for_element(root_element, context);

    let display_inside = match style.box_.display {
        Display::None => return ContainsFloats::No,
        Display::Contents if replaced.is_some() => {
            // 'display: contents' computes to 'none' for replaced elements
            return ContainsFloats::No;
        }
        // https://drafts.csswg.org/css-display-3/#transformations
        Display::Contents => DisplayInside::Flow,
        // The root element is blockified, ignore DisplayOutside
        Display::GeneratingBox(DisplayGeneratingBox::OutsideInside { inside, .. }) => inside,
    };

    if let Some(replaced) = replaced {
        match *replaced {}
        #[allow(unreachable_code)]
        {
            return ContainsFloats::No;
        }
    }

    if style.box_.position.is_absolutely_positioned() {
        boxes.push(BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(
            AbsolutelyPositionedBox {
                contents: IndependentFormattingContext::build(
                    context,
                    &style,
                    display_inside,
                    Contents::OfElement(root_element),
                ),
                style,
            },
        ));
        ContainsFloats::No
    } else if style.box_.float.is_floating() {
        boxes.push(BlockLevelBox::OutOfFlowFloatBox(FloatBox {
            contents: IndependentFormattingContext::build(
                context,
                &style,
                display_inside,
                Contents::OfElement(root_element),
            ),
            style,
        }));
        ContainsFloats::Yes
    } else {
        // FIXME: use `IndependentFormattingContext::build` and `BlockLevelBox::Independent`
        // once layout is implemented for the latter
        match display_inside {
            DisplayInside::Flow => {
                let (contents, contains_floats) = BlockContainer::build(
                    context,
                    &style,
                    NonReplacedContents::OfElement(root_element),
                );
                boxes.push(BlockLevelBox::SameFormattingContextBlock { style, contents });
                contains_floats
            }
        }
    }
}

enum IntermediateBlockLevelBox {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: IntermediateBlockContainer,
    },
    Independent {
        style: Arc<ComputedValues>,
        // FIXME: this should be IndependentFormattingContext:
        contents: Arc<ReplacedContent>,
    },
    OutOfFlowAbsolutelyPositionedBox {
        style: Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    },
    OutOfFlowFloatBox {
        style: Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
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
    Deferred { contents: NonReplacedContents },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContainsFloats {
    No,
    Yes,
}

impl BitOrAssign for ContainsFloats {
    fn bitor_assign(&mut self, other: Self) {
        if other == ContainsFloats::Yes {
            *self = ContainsFloats::Yes;
        }
    }
}

impl Default for ContainsFloats {
    fn default() -> Self {
        ContainsFloats::No
    }
}

impl IndependentFormattingContext {
    fn build<'a>(
        context: &'a Context<'a>,
        style: &'a Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    ) -> Self {
        match contents.try_into() {
            Ok(non_replaced) => match display_inside {
                DisplayInside::Flow => IndependentFormattingContext::Flow(
                    BlockFormattingContext::build(context, style, non_replaced),
                ),
            },
            Err(replaced) => IndependentFormattingContext::Replaced(replaced),
        }
    }
}

impl BlockFormattingContext {
    fn build<'a>(
        context: &'a Context<'a>,
        style: &'a Arc<ComputedValues>,
        contents: NonReplacedContents,
    ) -> Self {
        let (contents, contains_floats) = BlockContainer::build(context, style, contents);
        Self {
            contents,
            contains_floats: contains_floats == ContainsFloats::Yes,
        }
    }
}

/// A builder for a block container.
///
/// This builder starts from the first child of a given DOM node
/// and does a preorder traversal of all of its inclusive siblings.
struct BlockContainerBuilder<'a> {
    context: &'a Context<'a>,
    block_container_style: &'a Arc<ComputedValues>,

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

    /// The ongoing stack of inline boxes stack of the builder.
    ///
    /// Contains all the currently ongoing inline boxes we entered so far.
    /// The traversal is at all times as deep in the tree as this stack is,
    /// which is why the code doesn't need to keep track of the actual
    /// container root (see `handle_inline_level_element`).
    ///
    /// Whenever the end of a DOM element that represents an inline box is
    /// reached, the inline box at the top of this stack is complete and ready
    /// to be pushed to the children of the next last ongoing inline box
    /// the ongoing inline formatting context if the stack is now empty,
    /// which means we reached the end of a child of the actual
    /// container root (see `move_to_next_sibling`).
    ongoing_inline_boxes_stack: Vec<InlineBox>,

    /// The style of the anonymous block boxes pushed to the list of block-level
    /// boxes, if any (see `end_ongoing_inline_formatting_context`).
    anonymous_style: Option<Arc<ComputedValues>>,

    /// Whether the resulting block container contains any float box.
    contains_floats: ContainsFloats,
}

impl BlockContainer {
    fn build<'a>(
        context: &'a Context<'a>,
        block_container_style: &'a Arc<ComputedValues>,
        contents: NonReplacedContents,
    ) -> (BlockContainer, ContainsFloats) {
        let mut builder = BlockContainerBuilder {
            context,
            block_container_style,
            block_level_boxes: Default::default(),
            ongoing_inline_formatting_context: Default::default(),
            ongoing_inline_boxes_stack: Default::default(),
            anonymous_style: Default::default(),
            contains_floats: Default::default(),
        };

        contents.traverse(block_container_style, context, &mut builder);

        debug_assert!(builder.ongoing_inline_boxes_stack.is_empty());

        if !builder
            .ongoing_inline_formatting_context
            .inline_level_boxes
            .is_empty()
        {
            if builder.block_level_boxes.is_empty() {
                let container = BlockContainer::InlineFormattingContext(
                    builder.ongoing_inline_formatting_context,
                );
                return (container, builder.contains_floats);
            }
            builder.end_ongoing_inline_formatting_context();
        }

        let mut contains_floats = builder.contains_floats;
        let container = BlockContainer::BlockLevelBoxes(
            builder
                .block_level_boxes
                .into_par_iter()
                .mapfold_reduce_into(
                    &mut contains_floats,
                    |contains_floats, intermediate| {
                        let (block_level_box, box_contains_floats) = intermediate.finish(context);
                        *contains_floats |= box_contains_floats;
                        block_level_box
                    },
                    |left, right| *left |= right,
                )
                .collect(),
        );
        (container, contains_floats)
    }
}

impl<'a> dom::traversal::Handler for BlockContainerBuilder<'a> {
    fn handle_element(
        &mut self,
        style: &Arc<ComputedValues>,
        display: DisplayGeneratingBox,
        contents: Contents,
    ) {
        match display {
            DisplayGeneratingBox::OutsideInside { outside, inside } => match outside {
                DisplayOutside::Inline => self.handle_inline_level_element(style, inside, contents),
                DisplayOutside::Block => {
                    // Floats and abspos cause blockification, so they only happen in this case.
                    // https://drafts.csswg.org/css2/visuren.html#dis-pos-flo
                    if style.box_.position.is_absolutely_positioned() {
                        self.handle_absolutely_positioned_element(style.clone(), inside, contents)
                    } else if style.box_.float.is_floating() {
                        self.handle_float_element(style.clone(), inside, contents)
                    } else {
                        self.handle_block_level_element(style.clone(), inside, contents)
                    }
                }
            },
        }
    }

    fn handle_text(&mut self, input: &str, parent_style: &Arc<ComputedValues>) {
        let (leading_whitespace, mut input) = self.handle_leading_whitespace(input);
        if leading_whitespace || !input.is_empty() {
            // This text node should be pushed either to the next ongoing
            // inline level box with the parent style of that inline level box
            // that will be ended, or directly to the ongoing inline formatting
            // context with the parent style of that builder.
            let inlines = self.current_inline_level_boxes();

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
                let parent_style = parent_style.clone();
                inlines.push(InlineLevelBox::TextRun(TextRun { parent_style, text }))
            }
        }
    }
}

impl<'a> BlockContainerBuilder<'a> {
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
        let mut inline_level_boxes = self.current_inline_level_boxes().iter().rev();
        let mut stack = Vec::new();
        let preserved = loop {
            match inline_level_boxes.next() {
                Some(InlineLevelBox::TextRun(r)) => break !r.text.ends_with(' '),
                Some(InlineLevelBox::Atomic { .. }) => break false,
                Some(InlineLevelBox::OutOfFlowAbsolutelyPositionedBox(_))
                | Some(InlineLevelBox::OutOfFlowFloatBox(_)) => {}
                Some(InlineLevelBox::InlineBox(b)) => {
                    stack.push(inline_level_boxes);
                    inline_level_boxes = b.children.iter().rev()
                }
                None => {
                    if let Some(iter) = stack.pop() {
                        inline_level_boxes = iter
                    } else {
                        break false; // Paragraph start
                    }
                }
            }
        };
        let text = text.trim_start_matches(|c: char| c.is_ascii_whitespace());
        (preserved, text)
    }

    fn handle_inline_level_element(
        &mut self,
        style: &Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    ) {
        match contents.try_into() {
            Err(replaced) => {
                self.current_inline_level_boxes()
                    .push(InlineLevelBox::Atomic {
                        style: style.clone(),
                        contents: replaced,
                    });
            }
            Ok(non_replaced) => match display_inside {
                DisplayInside::Flow => {
                    // Whatever happened before, we just found an inline level element, so
                    // all we need to do is to remember this ongoing inline level box.
                    self.ongoing_inline_boxes_stack.push(InlineBox {
                        style: style.clone(),
                        first_fragment: true,
                        last_fragment: false,
                        children: vec![],
                    });

                    NonReplacedContents::traverse(non_replaced, &style, self.context, self);

                    let mut inline_box = self
                        .ongoing_inline_boxes_stack
                        .pop()
                        .expect("no ongoing inline level box found");
                    inline_box.last_fragment = true;
                    self.current_inline_level_boxes()
                        .push(InlineLevelBox::InlineBox(inline_box));
                }
            },
        }
    }

    fn handle_block_level_element(
        &mut self,
        style: Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    ) {
        // We just found a block level element, all ongoing inline level boxes
        // need to be split around it. We iterate on the fragmented inline
        // level box stack to take their contents and set their first_fragment
        // field to false, for the fragmented inline level boxes that will
        // come after the block level element.
        let mut fragmented_inline_boxes =
            self.ongoing_inline_boxes_stack
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

        if let Some(last) = fragmented_inline_boxes.next() {
            // There were indeed some ongoing inline level boxes before
            // the block, we accumulate them as a single inline level box
            // to be pushed to the ongoing inline formatting context.
            let mut fragmented_inline = InlineLevelBox::InlineBox(last);
            for mut fragmented_parent_inline_box in fragmented_inline_boxes {
                fragmented_parent_inline_box
                    .children
                    .push(fragmented_inline);
                fragmented_inline = InlineLevelBox::InlineBox(fragmented_parent_inline_box);
            }

            self.ongoing_inline_formatting_context
                .inline_level_boxes
                .push(fragmented_inline);
        }

        // We found a block level element, so the ongoing inline formatting
        // context needs to be ended.
        self.end_ongoing_inline_formatting_context();

        self.block_level_boxes.push(match contents.try_into() {
            Ok(contents) => match display_inside {
                DisplayInside::Flow => IntermediateBlockLevelBox::SameFormattingContextBlock {
                    style,
                    contents: IntermediateBlockContainer::Deferred { contents },
                },
            },
            Err(contents) => IntermediateBlockLevelBox::Independent { style, contents },
        })
    }

    fn handle_absolutely_positioned_element(
        &mut self,
        style: Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    ) {
        if !self.has_ongoing_inline_formatting_context() {
            let box_ = IntermediateBlockLevelBox::OutOfFlowAbsolutelyPositionedBox {
                style,
                contents,
                display_inside,
            };
            self.block_level_boxes.push(box_)
        } else {
            let box_ = InlineLevelBox::OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox {
                contents: IndependentFormattingContext::build(
                    self.context,
                    &style,
                    display_inside,
                    contents,
                ),
                style,
            });
            self.current_inline_level_boxes().push(box_)
        }
    }

    fn handle_float_element(
        &mut self,
        style: Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    ) {
        self.contains_floats = ContainsFloats::Yes;

        if !self.has_ongoing_inline_formatting_context() {
            let box_ = IntermediateBlockLevelBox::OutOfFlowFloatBox {
                style,
                contents,
                display_inside,
            };
            self.block_level_boxes.push(box_);
        } else {
            let box_ = InlineLevelBox::OutOfFlowFloatBox(FloatBox {
                contents: IndependentFormattingContext::build(
                    self.context,
                    &style,
                    display_inside,
                    contents,
                ),
                style,
            });
            self.current_inline_level_boxes().push(box_);
        }
    }

    fn end_ongoing_inline_formatting_context(&mut self) {
        assert!(
            self.ongoing_inline_boxes_stack.is_empty(),
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

        let block_container_style = self.block_container_style;
        let anonymous_style = self.anonymous_style.get_or_insert_with(|| {
            // If parent_style is None, the parent is the document node,
            // in which case anonymous inline boxes should inherit their
            // styles from initial values.
            ComputedValues::anonymous_inheriting_from(Some(block_container_style))
        });

        self.block_level_boxes
            .push(IntermediateBlockLevelBox::SameFormattingContextBlock {
                style: anonymous_style.clone(),
                contents: IntermediateBlockContainer::InlineFormattingContext(
                    self.ongoing_inline_formatting_context.take(),
                ),
            });
    }

    fn current_inline_level_boxes(&mut self) -> &mut Vec<InlineLevelBox> {
        match self.ongoing_inline_boxes_stack.last_mut() {
            Some(last) => &mut last.children,
            None => &mut self.ongoing_inline_formatting_context.inline_level_boxes,
        }
    }

    fn has_ongoing_inline_formatting_context(&self) -> bool {
        !self
            .ongoing_inline_formatting_context
            .inline_level_boxes
            .is_empty()
            || !self.ongoing_inline_boxes_stack.is_empty()
    }
}

impl IntermediateBlockLevelBox {
    fn finish(self, context: &Context) -> (BlockLevelBox, ContainsFloats) {
        match self {
            IntermediateBlockLevelBox::SameFormattingContextBlock { style, contents } => {
                let (contents, contains_floats) = contents.finish(context, &style);
                let block_level_box = BlockLevelBox::SameFormattingContextBlock { contents, style };
                (block_level_box, contains_floats)
            }
            IntermediateBlockLevelBox::Independent { style, contents } => (
                BlockLevelBox::Independent { style, contents },
                ContainsFloats::No,
            ),
            IntermediateBlockLevelBox::OutOfFlowAbsolutelyPositionedBox {
                style,
                display_inside,
                contents,
            } => {
                let block_level_box =
                    BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox {
                        contents: IndependentFormattingContext::build(
                            context,
                            &style,
                            display_inside,
                            contents,
                        ),
                        style: style,
                    });
                (block_level_box, ContainsFloats::No)
            }
            IntermediateBlockLevelBox::OutOfFlowFloatBox {
                style,
                display_inside,
                contents,
            } => {
                let contents =
                    IndependentFormattingContext::build(context, &style, display_inside, contents);
                let block_level_box =
                    BlockLevelBox::OutOfFlowFloatBox(FloatBox { contents, style });
                (block_level_box, ContainsFloats::Yes)
            }
        }
    }
}

impl IntermediateBlockContainer {
    fn finish(
        self,
        context: &Context,
        style: &Arc<ComputedValues>,
    ) -> (BlockContainer, ContainsFloats) {
        match self {
            IntermediateBlockContainer::Deferred { contents } => {
                BlockContainer::build(context, style, contents)
            }
            IntermediateBlockContainer::InlineFormattingContext(ifc) => {
                // If that inline formatting context contained any float, those
                // were already taken into account during the first phase of
                // box construction.
                (
                    BlockContainer::InlineFormattingContext(ifc),
                    ContainsFloats::No,
                )
            }
        }
    }
}
