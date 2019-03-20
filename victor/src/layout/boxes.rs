use crate::style::ComputedValues;
use html5ever::tendril::StrTendril;
use std::sync::Arc;

mod generation;

pub(super) type BoxTreeRoot = BlockFormattingContext;

#[allow(unused)]
pub(super) enum FormattingContext {
    // Not included: inline formatting context, which is always part of a block container
    Flow(BlockFormattingContext),
    // Replaced(ReplacedElement), // Not called FC in specs, but behaves close enough
    // Table(Table),
    // Other layout modes go here
}

pub(super) struct BlockFormattingContext(pub BlockContainer);

pub(super) enum BlockContainer {
    BlockLevels(Vec<BlockLevel>),
    InlineFormattingContext(Vec<InlineLevel>),
}

pub(super) enum BlockLevel {
    #[allow(unused)]
    SameFormattingContextBlock {
        style: Arc<ComputedValues>,
        contents: BlockContainer,
    },
    // Other {
    //     style: Arc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}

pub(super) enum InlineLevel {
    Text(StrTendril),
    #[allow(unused)]
    Inline {
        style: Arc<ComputedValues>,
        first_fragment: bool,
        last_fragment: bool,
        children: Vec<InlineLevel>,
    },
    // Atomic {
    //     style: Arc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}
