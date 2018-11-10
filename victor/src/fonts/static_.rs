use crate::lazy_arc::LazyArc;
use std::sync::Arc;
use super::{Font, FontError};

/// Include a TrueType file with `include_bytes!()` and create a [`LazyStaticFont`] value.
///
/// This value can be used to initialize a `static` item:
///
/// ```rust
/// static MY_FONT: LazyStaticFont = include_font!("../my_font.ttf");
/// ```
///
/// [`LazyStaticFont`]: fonts/struct.LazyStaticFont.html
#[macro_export]
macro_rules! include_font {
    ($filename: expr) => {
        $crate::fonts::LazyStaticFont {
            bytes: include_bytes!($filename),
            lazy_arc: $crate::lazy_arc::LazyArc::INIT,
        }
    }
}

/// The regular sans-serif face of the [Bitstream Vera](https://www.gnome.org/fonts/) font family.
pub static BITSTREAM_VERA_SANS: LazyStaticFont = include_font!("../../fonts/vera/Vera.ttf");

/// A lazily-parsed font backed by a static bytes slice.
pub struct LazyStaticFont {
    /// The raw data for this font
    pub bytes: &'static [u8],

    // This field needs to be public so that static initializers can construct it.
    // A `const fn` constructor would be better,
    // but these are not avaiable on stable as of this writing.
    #[doc(hidden)] pub lazy_arc: LazyArc<Font>,
}

impl LazyStaticFont {
    // FIXME: figure out minimal Ordering for atomic operations

    /// Return a new `Arc` reference to the singleton `Font` object.
    ///
    /// If this font’s singleton was not already initialized,
    /// try to parse the font now (this may return an error) to initialize it.
    ///
    /// Calling this reapeatedly will only parse once (until `.drop()` is called).
    pub fn get(&self) -> Result<Arc<Font>, FontError> {
        self.lazy_arc.get_or_create(|| Font::parse(self.bytes))
    }

// Oops, this turned out to be unsound. See victor/src/lazy_arc/mod.rs

//    /// Deinitialize this font’s singleton, dropping the internal `Arc` reference.
//    ///
//    /// Calling `.get()` again afterwards will parse a new `Font` object.
//    ///
//    /// The previous `Font` object may continue to live as long
//    /// as other `Arc` references to it exist.
//    pub fn drop(&self) {
//        self.lazy_arc.drop()
//    }
}
