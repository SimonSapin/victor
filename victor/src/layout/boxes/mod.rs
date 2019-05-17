use crate::style::ComputedValues;
use std::sync::Arc;

mod construct;

pub(super) type BoxTreeRoot = BlockFormattingContext;

/// https://drafts.csswg.org/css-display/#independent-formatting-context
#[derive(Debug)]
pub(super) enum IndependentFormattingContext {
    Flow(BlockFormattingContext),

    // Not called FC in specs, but behaves close enough
    Replaced(Arc<ReplacedContent>),
    // Other layout modes go here
}

#[derive(Debug)]
pub(super) struct BlockFormattingContext {
    pub contents: BlockContainer,
    pub contains_floats: bool,
}

#[derive(Debug)]
pub(super) enum BlockContainer {
    BlockLevelBoxes(Vec<BlockLevelBox>),
    InlineFormattingContext(InlineFormattingContext),
}

#[derive(Debug)]
pub(super) enum BlockLevelBox {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: BlockContainer,
    },
    OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox),
    OutOfFlowFloatBox(FloatBox),
    // Independent {
    //     style: Arc<ComputedValues>,
    //     contents: IndependentFormattingContext,
    // },
}

#[derive(Debug)]
pub(super) struct AbsolutelyPositionedBox {
    pub style: Arc<ComputedValues>,
    pub contents: IndependentFormattingContext,
}

#[derive(Debug)]
pub(super) struct FloatBox {
    pub style: Arc<ComputedValues>,
    pub contents: IndependentFormattingContext,
}

#[derive(Debug, Default)]
pub(super) struct InlineFormattingContext {
    pub inline_level_boxes: Vec<InlineLevelBox>,
}

#[derive(Debug)]
pub(super) enum InlineLevelBox {
    InlineBox(InlineBox),
    TextRun(TextRun),
    OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox),
    OutOfFlowFloatBox(FloatBox),
    Atomic {
        style: Arc<ComputedValues>,
        // FIXME: this should be IndependentFormattingContext:
        contents: Arc<ReplacedContent>,
    },
}

#[derive(Debug)]
pub(super) struct InlineBox {
    pub style: Arc<ComputedValues>,
    pub first_fragment: bool,
    pub last_fragment: bool,
    pub children: Vec<InlineLevelBox>,
}

/// https://www.w3.org/TR/css-display-3/#css-text-run
#[derive(Debug)]
pub(super) struct TextRun {
    pub parent_style: Arc<ComputedValues>,
    pub text: String,
}

#[derive(Debug)]
pub(crate) enum ReplacedContent {
    // Not implemented yet
}
