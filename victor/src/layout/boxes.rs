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
    BlockLevels(Vec<BlockLevel>),
    InlineFormattingContext(InlineFormattingContext),
}

#[derive(Debug)]
pub(super) enum BlockLevel {
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: BlockContainer,
    },
    // Other {
    //     style: Arc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}

#[derive(Debug)]
pub(super) struct InlineFormattingContext {
    inline_level_boxes: Vec<InlineLevel>,
    text: Vec<Text>,
}

#[derive(Debug)]
pub(super) enum InlineLevel {
    #[allow(unused)]
    Box(InlineLevelBox),
    #[allow(unused)]
    Text(TextId),
    // Atomic {
    //     style: Arc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}

#[derive(Debug)]
pub(super) struct InlineLevelBox {
    style: Arc<ComputedValues>,
    first_fragment: bool,
    last_fragment: bool,
    children: Vec<InlineLevel>,
}

#[derive(Debug)]
pub(super) struct TextId(usize);

#[derive(Debug)]
pub(super) struct Text {
    parent_style: Arc<ComputedValues>,
    segment: ShapedSegment,
}
