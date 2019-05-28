use super::*;

#[derive(Debug)]
pub(in crate::layout) struct FloatBox {
    pub style: Arc<ComputedValues>,
    pub contents: IndependentFormattingContext,
}
