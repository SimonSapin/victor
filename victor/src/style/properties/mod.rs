use style::values::length::{Length, PxLength};
use style::values::generic::FourSides;

#[macro_use]
mod macros;

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

    @shorthands {
        "margin": parse_four_sides!(margin_top, margin_left, margin_bottom, margin_right);
    }
}
