pub(crate) use crate::style::values::Length;

pub(crate) mod physical {
    #[derive(Debug, Clone)]
    pub(crate) struct Sides<T> {
        pub top: T,
        pub left: T,
        pub bottom: T,
        pub right: T,
    }
}

pub(crate) mod flow_relative {
    #[derive(Debug, Clone)]
    pub(crate) struct Vec2<T> {
        pub inline: T,
        pub block: T,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Sides<T> {
        pub inline_start: T,
        pub inline_end: T,
        pub block_start: T,
        pub block_end: T,
    }
}

use crate::style::values::{Direction, WritingMode};
use std::ops::Add;

impl<T: Clone> physical::Sides<T> {
    pub fn to_flow_relative(&self, mode: (WritingMode, Direction)) -> flow_relative::Sides<T> {
        use Direction::*;
        use WritingMode::*;

        // https://drafts.csswg.org/css-writing-modes/#logical-to-physical
        let (bs, be) = match mode.0 {
            HorizontalTb => (&self.top, &self.bottom),
            VerticalRl | SidewaysRl => (&self.right, &self.left),
            VerticalLr | SidewaysLr => (&self.left, &self.right),
        };
        let (is, ie) = match mode {
            (HorizontalTb, Ltr) => (&self.left, &self.right),
            (HorizontalTb, Rtl) => (&self.right, &self.left),
            (VerticalRl, Ltr) | (SidewaysRl, Ltr) | (VerticalLr, Ltr) | (SidewaysLr, Rtl) => {
                (&self.top, &self.bottom)
            }
            (VerticalRl, Rtl) | (SidewaysRl, Rtl) | (VerticalLr, Rtl) | (SidewaysLr, Ltr) => {
                (&self.bottom, &self.top)
            }
        };
        flow_relative::Sides {
            inline_start: is.clone(),
            inline_end: ie.clone(),
            block_start: bs.clone(),
            block_end: be.clone(),
        }
    }
}

impl<T> flow_relative::Sides<T> {
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> flow_relative::Sides<U> {
        flow_relative::Sides {
            inline_start: f(&self.inline_start),
            inline_end: f(&self.inline_end),
            block_start: f(&self.block_start),
            block_end: f(&self.block_end),
        }
    }

    pub fn map_block_inline_axes<U>(
        &self,
        inline_f: impl Fn(&T) -> U,
        block_f: impl Fn(&T) -> U,
    ) -> flow_relative::Sides<U> {
        flow_relative::Sides {
            inline_start: inline_f(&self.inline_start),
            inline_end: inline_f(&self.inline_end),
            block_start: block_f(&self.block_start),
            block_end: block_f(&self.block_end),
        }
    }

    pub fn inline_sum(&self) -> <&T as Add>::Output
    where
        for<'a> &'a T: Add,
    {
        &self.inline_start + &self.inline_end
    }

    pub fn block_sum(&self) -> <&T as Add>::Output
    where
        for<'a> &'a T: Add,
    {
        &self.block_start + &self.block_end
    }

    pub fn start_corner(&self) -> flow_relative::Vec2<T>
    where
        T: Clone,
    {
        flow_relative::Vec2 {
            inline: self.inline_start.clone(),
            block: self.block_start.clone(),
        }
    }
}

impl<'a, 'b, T, U> Add<&'b flow_relative::Sides<U>> for &'a flow_relative::Sides<T>
where
    T: Add<U> + Copy,
    U: Copy,
{
    type Output = flow_relative::Sides<T::Output>;

    fn add(self, other: &'b flow_relative::Sides<U>) -> Self::Output {
        flow_relative::Sides {
            inline_start: self.inline_start + other.inline_start,
            inline_end: self.inline_end + other.inline_end,
            block_start: self.block_start + other.block_start,
            block_end: self.block_end + other.block_end,
        }
    }
}
