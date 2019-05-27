use crate::dom;
use crate::dom::traversal::{Contents, Context, NonReplacedContents};
use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::values::*;
use crate::style::{style_for_element, ComputedValues};
use std::convert::TryInto;
use std::sync::Arc;

mod flow;
mod fragments;
mod positioned;
mod replaced;

use flow::*;
pub(crate) use fragments::*;
use positioned::*;
pub(crate) use replaced::*;

/// https://drafts.csswg.org/css-display/#independent-formatting-context
#[derive(Debug)]
enum IndependentFormattingContext {
    Flow(BlockFormattingContext),

    // Not called FC in specs, but behaves close enough
    Replaced(ReplacedContent),
    // Other layout modes go here
}

impl IndependentFormattingContext {
    fn construct<'a>(
        context: &'a Context<'a>,
        style: &'a Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
    ) -> Self {
        match contents.try_into() {
            Ok(non_replaced) => match display_inside {
                DisplayInside::Flow => IndependentFormattingContext::Flow(
                    BlockFormattingContext::construct(context, style, non_replaced),
                ),
            },
            Err(replaced) => IndependentFormattingContext::Replaced(replaced),
        }
    }
}

struct ContainingBlock {
    inline_size: Length,
    block_size: Option<Length>,
    mode: (WritingMode, Direction),
}

/// https://drafts.csswg.org/css2/visuren.html#relative-positioning
fn relative_adjustement(
    style: &ComputedValues,
    inline_size: Length,
    block_size: Option<Length>,
) -> Vec2<Length> {
    if !style.box_.position.is_relatively_positioned() {
        return Vec2::zero();
    }
    fn adjust(start: Option<Length>, end: Option<Length>) -> Length {
        match (start, end) {
            (None, None) => Length::zero(),
            (Some(start), _) => start,
            (None, Some(end)) => -end,
        }
    }
    let block_size = block_size.unwrap_or(Length::zero());
    let box_offsets = style.box_offsets().map_inline_and_block_axes(
        |v| v.non_auto().map(|v| v.percentage_relative_to(inline_size)),
        |v| v.non_auto().map(|v| v.percentage_relative_to(block_size)),
    );
    Vec2 {
        inline: adjust(box_offsets.inline_start, box_offsets.inline_end),
        block: adjust(box_offsets.block_start, box_offsets.block_end),
    }
}

trait Take {
    fn take(&mut self) -> Self;
}

impl<T> Take for T
where
    T: Default,
{
    fn take(&mut self) -> Self {
        std::mem::replace(self, Default::default())
    }
}
