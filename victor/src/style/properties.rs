use crate::style::values::border::*;
use crate::style::values::generic::FourSides;
use crate::style::values::length::{Length, PxLength};
use crate::style::values::*;
use cssparser::Color;

// `include` rather than `mod` so that macro definition and use are in the same scope,
// which makes `use` imports easier.
include!("properties/macros.rs");

properties! {
    type Discriminant = u8;

    inherited struct font {
        font_size {
            "font-size",
            Length,
            initial = PxLength::new(16.)
        }
    }

    reset struct margin {
        margin_top { "margin-top", Length, initial = PxLength::new(0.) }
        margin_left { "margin-left", Length, initial = PxLength::new(0.) }
        margin_bottom { "margin-bottom", Length, initial = PxLength::new(0.) }
        margin_right { "margin-right", Length, initial = PxLength::new(0.) }
    }

    reset struct border {
        border_top_color { "border-top-color", Color, initial = Color::CurrentColor }
        border_left_color { "border-left-color", Color, initial = Color::CurrentColor }
        border_bottom_color { "border-bottom-color", Color, initial = Color::CurrentColor }
        border_right_color { "border-color-right", Color, initial = Color::CurrentColor }

        border_top_style { "border-top-style", LineStyle, initial = LineStyle::None }
        border_left_style { "border-left-style", LineStyle, initial = LineStyle::None }
        border_bottom_style { "border-bottom-style", LineStyle, initial = LineStyle::None }
        border_right_style { "border-right-style", LineStyle, initial = LineStyle::None }

        border_top_width { "border-top-width", LineWidth, initial = LineWidth::MEDIUM }
        border_left_width { "border-left-width", LineWidth, initial = LineWidth::MEDIUM }
        border_bottom_width { "border-bottom-width", LineWidth, initial = LineWidth::MEDIUM }
        border_right_width { "border-right-width", LineWidth, initial = LineWidth::MEDIUM }
    }

    reset struct display {
        display {
            "display",
            Display,
            initial = Display::Other {
                outside: DisplayOutside::Inline,
                inside: DisplayInside::Flow,
            }
        }
    }

    @shorthands {
        "margin" => FourSides<Length> {
            top: margin_top,
            left: margin_left,
            bottom: margin_bottom,
            right: margin_right,
        }
        "border-style" => FourSides<LineStyle> {
            top: border_top_style,
            left: border_left_style,
            bottom: border_bottom_style,
            right: border_right_style,
        }
        "border-color" => FourSides<Color> {
            top: border_top_color,
            left: border_left_color,
            bottom: border_bottom_color,
            right: border_right_color,
        }
        "border-width" => FourSides<LineWidth> {
            top: border_top_width,
            left: border_left_width,
            bottom: border_bottom_width,
            right: border_right_width,
        }
        "border-top" => BorderSide {
            style: border_top_style,
            color: border_top_color,
            width: border_top_width,
        }
    }
}
