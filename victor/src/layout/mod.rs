use html5ever::tendril::StrTendril;

mod box_generation;

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
    InlineFormattingContext(Vec<InlineLevel>),
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
