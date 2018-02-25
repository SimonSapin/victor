use primitives::{Length as EuclidLength};
use style::values::Length;

#[macro_use]
#[path = "properties_macro.rs"]
mod properties_macro;

properties! {
    type Discriminant = u8;

    font_size {
        name: "font-size",
        specified: Length,
        initial: EuclidLength::new(16.),
    }
}
