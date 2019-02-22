use crate::geom::flow_relative::{Sides, Vec2};
use crate::geom::Length;
use crate::style::ComputedValues;
use std::rc::Rc;

pub(crate) struct Fragment {
    pub style: Rc<ComputedValues>,
    pub children: Vec<Fragment>,

    /// From the containing block’s start corner
    pub content_start_corner: Vec2<Length>,

    pub content_size: Vec2<Length>,
    pub padding: Sides<Length>,
    pub border: Sides<Length>,
    pub margin: Sides<Length>,
}
