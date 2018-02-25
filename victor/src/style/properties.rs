use primitives::{Length as EuclidLength};
use style::values::Length;

#[macro_use]
#[path = "properties_macro.rs"]
mod properties_macro;

properties! {
    type Discriminant = u8;

    inherited struct font {
        font_size {
            name: "font-size",
            specified: Length,
            initial: EuclidLength::new(16.),
        }
    }

    reset struct margin {
        margin {
            name: "margin",
            specified: Length,  // FIXME: shorthand, 4 values
            initial: EuclidLength::new(0.),
        }
    }
}
