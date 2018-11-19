use crate::style::ComputedValues;
use html5ever::tendril::StrTendril;
use std::rc::Rc;

mod box_generation;

type BoxTreeRoot = BlockFormattingContext;

#[allow(unused)]
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
    #[allow(unused)]
    SameFormattingContextBlock {
        style: Rc<ComputedValues>,
        contents: BlockContainer,
    },
    // Other {
    //     style: Rc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}

enum InlineLevel {
    Text(StrTendril),
    #[allow(unused)]
    Inline {
        style: Rc<ComputedValues>,
        children: Vec<InlineLevel>,
    },
    // Atomic {
    //     style: Rc<ComputedValues>,
    //     contents: FormattingContext,
    // },
}
