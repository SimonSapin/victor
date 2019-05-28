use super::*;

#[derive(Debug)]
pub(in crate::layout) struct FloatBox {
    pub style: Arc<ComputedValues>,
    pub contents: IndependentFormattingContext,
    pub intrinsic_sizes: IntrinsicSizes,
}

impl FloatBox {
    pub(in crate::layout) fn needs_intrinsic_sizes(style: &ComputedValues) -> bool {
        matches!(style.box_size().inline, LengthOrPercentageOrAuto::Auto)
    }
}

/// Data kept during layout about the floats in a given block formatting context.
pub(in crate::layout) struct FloatContext {
    // TODO
}

impl FloatContext {
    pub fn new() -> Self {
        FloatContext {}
    }
}
