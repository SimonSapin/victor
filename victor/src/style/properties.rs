use primitives::{Length as EuclidLength};
use style::values::{Parse, Length, FourSides};

#[macro_use]
#[path = "properties_macros.rs"]
mod properties_macros;

properties! {
    type Discriminant = u8;

    inherited struct font {
        font_size {
            "font-size",
            Length,
            initial = EuclidLength::new(16.)
        }
    }

    reset struct margin {
        margin_top { "margin-top", Length, initial = EuclidLength::new(0.) }
        margin_left { "margin-left", Length, initial = EuclidLength::new(0.) }
        margin_bottom { "margin-bottom", Length, initial = EuclidLength::new(0.) }
        margin_right { "margin-right", Length, initial = EuclidLength::new(0.) }
    }

    @shorthands {
        "margin": parse_four_sides!(margin_top, margin_left, margin_bottom, margin_right);
    }
}
