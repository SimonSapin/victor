use crate::dom;
use crate::geom::flow_relative::{Rect, Sides, Vec2};
use crate::geom::Length;
use crate::style::values::*;
use crate::style::{style_for_element, ComputedValues};
use std::convert::TryInto;
use std::sync::Arc;

mod dom_traversal;
mod element_data;
mod flow;
mod fragments;
mod positioned;
mod replaced;

use dom_traversal::*;
use flow::*;
use positioned::*;
use replaced::*;

pub(crate) use element_data::*;
pub(crate) use fragments::*;

/// https://drafts.csswg.org/css-display/#independent-formatting-context
#[derive(Debug)]
enum IndependentFormattingContext {
    Flow(BlockFormattingContext),

    // Not called FC in specs, but behaves close enough
    Replaced(ReplacedContent),
    // Other layout modes go here
}

enum NonReplacedIFC<'a> {
    Flow(&'a BlockFormattingContext),
}

impl IndependentFormattingContext {
    fn construct<'a>(
        context: &'a Context<'a>,
        style: &'a Arc<ComputedValues>,
        display_inside: DisplayInside,
        contents: Contents,
        request_intrinsic_sizes: bool,
    ) -> (Self, IntrinsicSizes) {
        match contents.try_into() {
            Ok(non_replaced) => match display_inside {
                DisplayInside::Flow | DisplayInside::FlowRoot => {
                    let (bfc, intrinsic_sizes) = BlockFormattingContext::construct(
                        context,
                        style,
                        non_replaced,
                        request_intrinsic_sizes,
                    );
                    (IndependentFormattingContext::Flow(bfc), intrinsic_sizes)
                }
            },
            Err(replaced) => {
                let intrinsic_sizes = replaced.unimplemented();
                (
                    IndependentFormattingContext::Replaced(replaced),
                    intrinsic_sizes,
                )
            }
        }
    }

    fn as_replaced(&self) -> Result<&ReplacedContent, NonReplacedIFC> {
        match self {
            IndependentFormattingContext::Replaced(r) => Ok(r),
            IndependentFormattingContext::Flow(f) => Err(NonReplacedIFC::Flow(f)),
        }
    }

    fn layout<'a>(
        &'a self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    ) -> FlowChildren {
        match self.as_replaced() {
            Ok(replaced) => replaced.unimplemented(),
            Err(ifc) => ifc.layout(containing_block, tree_rank, absolutely_positioned_fragments),
        }
    }
}

impl<'a> NonReplacedIFC<'a> {
    fn layout(
        &self,
        containing_block: &ContainingBlock,
        tree_rank: usize,
        absolutely_positioned_fragments: &mut Vec<AbsolutelyPositionedFragment<'a>>,
    ) -> FlowChildren {
        match self {
            NonReplacedIFC::Flow(bfc) => {
                bfc.layout(containing_block, tree_rank, absolutely_positioned_fragments)
            }
        }
    }
}

struct ContainingBlock {
    inline_size: Length,
    block_size: LengthOrAuto,
    mode: (WritingMode, Direction),
}

struct DefiniteContainingBlock {
    size: Vec2<Length>,
    mode: (WritingMode, Direction),
}

#[derive(Debug, Clone)]
enum IntrinsicSizes {
    WasNotRequested,
    Available {
        min_content: Length,
        max_content: Length,
    },
}

impl Default for IntrinsicSizes {
    fn default() -> Self {
        IntrinsicSizes::WasNotRequested
    }
}

impl IntrinsicSizes {
    fn shrink_to_fit(&self, available_inline_size: Length) -> Length {
        match *self {
            IntrinsicSizes::WasNotRequested => {
                panic!("Using shrink-to-fit without requesting intrinsic sizes")
            }
            IntrinsicSizes::Available {
                min_content,
                max_content,
            } => available_inline_size.max(min_content).min(max_content),
        }
    }

    fn zero_if_requested(requested: bool) -> Self {
        if requested {
            IntrinsicSizes::Available {
                min_content: Length::zero(),
                max_content: Length::zero(),
            }
        } else {
            IntrinsicSizes::WasNotRequested
        }
    }

    fn max(&self, other: Self) -> Self {
        match *self {
            IntrinsicSizes::WasNotRequested => other,
            IntrinsicSizes::Available {
                min_content: min_self,
                max_content: max_self,
            } => match other {
                IntrinsicSizes::WasNotRequested => self.clone(),
                IntrinsicSizes::Available {
                    min_content: min_other,
                    max_content: max_other,
                } => IntrinsicSizes::Available {
                    min_content: min_self.max(min_other),
                    max_content: max_self.max(max_other),
                },
            },
        }
    }
}

/// https://drafts.csswg.org/css2/visuren.html#relative-positioning
fn relative_adjustement(
    style: &ComputedValues,
    inline_size: Length,
    block_size: LengthOrAuto,
) -> Vec2<Length> {
    if !style.box_.position.is_relatively_positioned() {
        return Vec2::zero();
    }
    fn adjust(start: LengthOrAuto, end: LengthOrAuto) -> Length {
        match (start, end) {
            (LengthOrAuto::Auto, LengthOrAuto::Auto) => Length::zero(),
            (LengthOrAuto::Auto, LengthOrAuto::Length(end)) => -end,
            (LengthOrAuto::Length(start), _) => start,
        }
    }
    let block_size = block_size.auto_is(Length::zero);
    let box_offsets = style.box_offsets().map_inline_and_block_axes(
        |v| v.percentage_relative_to(inline_size),
        |v| v.percentage_relative_to(block_size),
    );
    Vec2 {
        inline: adjust(box_offsets.inline_start, box_offsets.inline_end),
        block: adjust(box_offsets.block_start, box_offsets.block_end),
    }
}

// FIXME: use std::mem::take when it’s stable
// https://github.com/rust-lang/rust/issues/61129
fn take<T>(x: &mut T) -> T
where
    T: Default,
{
    std::mem::replace(x, Default::default())
}
