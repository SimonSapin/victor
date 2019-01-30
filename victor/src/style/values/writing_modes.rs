#[derive(Debug, Copy, Clone, PartialEq, Parse)]
pub(crate) enum Direction {
    Ltr,
    Rtl,
}

#[derive(Debug, Copy, Clone, PartialEq, Parse)]
pub(crate) enum WritingMode {
    HorizontalTb,
    VerticalRl,
    VerticalLr,
    SidewaysRl,
    SidewaysLr,
}
