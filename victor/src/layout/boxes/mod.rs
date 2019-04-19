use crate::style::ComputedValues;
use crate::text::ShapedSegment;
use std::sync::Arc;

mod generation;

pub(super) type BoxTreeRoot = BlockFormattingContext;

#[allow(unused)]
#[derive(Debug)]
pub(super) enum FormattingContext {
    // Not included: inline formatting context, which is always part of a block container
    Flow(BlockFormattingContext),
    // Replaced(ReplacedElement), // Not called FC in specs, but behaves close enough
    // Table(Table),
    // Other layout modes go here
}

#[derive(Debug)]
pub(super) struct BlockFormattingContext(pub BlockContainer);

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
    // Other {
    //     style: Arc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}

#[derive(Debug)]
pub(super) struct AbsolutelyPositionedBox {
    pub style: Arc<ComputedValues>,
    pub contents: BlockFormattingContext,
}

#[derive(Debug)]
pub(super) struct InlineFormattingContext {
    pub inline_level_boxes: Vec<InlineLevelBox>,
    pub text_runs: Vec<TextRun>,
}

#[derive(Debug)]
pub(super) enum InlineLevelBox {
    InlineBox(InlineBox),
    TextRun(TextRunId),
    OutOfFlowAbsolutelyPositionedBox(AbsolutelyPositionedBox),
    // Atomic {
    //     style: Arc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}

#[derive(Debug)]
pub(super) struct InlineBox {
    pub style: Arc<ComputedValues>,
    pub first_fragment: bool,
    pub last_fragment: bool,
    pub children: Vec<InlineLevelBox>,
}

#[derive(Debug)]
pub(super) struct TextRunId(pub usize);

/// https://www.w3.org/TR/css-display-3/#css-text-run
///
/// Contiguous sequence of sibling text nodes generates multiple text runs,
/// as opposed as in the specification.
#[derive(Debug)]
pub(super) struct TextRun {
    pub parent_style: Arc<ComputedValues>,
    pub segment: ShapedSegment,
}
