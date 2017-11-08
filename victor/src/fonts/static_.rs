use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::mem;
use super::Font;
use raw_mutex::{RawMutex, RAW_MUTEX_INIT};

/// Include a TrueType file with `include_bytes!()` and create a [`LazyStaticFont`] value.
///
/// This value should be used to initialize a `static` item:
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
            private: $crate::fonts::LAZY_STATIC_FONT_PRIVATE_INIT,
        }
    }
}

/// The regular sans-serif face of the [Bitstream Vera](https://www.gnome.org/fonts/) font family.
pub static BITSTREAM_VERA_SANS: LazyStaticFont = include_font!("../../fonts/vera/Vera.ttf");

/// A lazily-parsed font backed by a static bytes slice.
pub struct LazyStaticFont {
    /// This font’s raw data
    pub bytes: &'static [u8],

    /// This field needs to be public so that static initializers can construct it.
    /// A `const fn` constructor would be better,
    /// but these are not avaiable on stable as of this writing.
    #[doc(hidden)]
    pub private: LazyStaticFontPrivate,
}

pub const LAZY_STATIC_FONT_PRIVATE_INIT: LazyStaticFontPrivate = LazyStaticFontPrivate {
    mutex: RAW_MUTEX_INIT,
    ptr: ATOMIC_USIZE_INIT,
};

pub struct LazyStaticFontPrivate {
    mutex: RawMutex,
    ptr: AtomicUsize,
}

impl LazyStaticFont {
    // FIXME: figure out minimal Ordering for atomic operations

    /// Return a new `Arc` reference to the singleton `Font` object.
    ///
    /// If this font’s singleton was not already initialized,
    /// try to parse the font now (this may return an error) to initialize it.
    ///
    /// Calling `$font_name().get()` reapeatedly will only parse once
    /// (until `.drop()` is called).
    pub fn get(&self) -> io::Result<Arc<Font>> {
        macro_rules! try_load {
            () => {
                let ptr = self.private.ptr.load(Ordering::SeqCst);
                if ptr != 0 {
                    // Already initialized
                    unsafe {
                        return Ok(clone_raw_arc(ptr))
                    }
                }
            }
        }

        // First try to obtain a font from the atomic pointer without taking the mutex
        try_load!();

        // Synchronize initialization
        struct RawMutexGuard<'a>(&'a RawMutex);
        impl<'a> Drop for RawMutexGuard<'a> {
            fn drop(&mut self) {
                self.0.unlock()
            }
        }

        self.private.mutex.lock();
        let _guard = RawMutexGuard(&self.private.mutex);

        // Try again in case some other thread raced us while we were taking the mutex
        try_load!();

        // Now we’ve observed the atomic pointer uninitialized after taking the mutex:
        // we’re definitely first

        let font = Font::from_bytes(self.bytes)?;
        let new_ptr = Arc::into_raw(font.clone()) as usize;
        self.private.ptr.store(new_ptr, Ordering::SeqCst);
        Ok(font)
    }

    /// Deinitialize this font’s singleton, dropping the internal `Arc` reference.
    ///
    /// Calling `.get()` again afterwards will parse a new `Font` object.
    ///
    /// The previous `Font` object may continue to live as long
    /// as other `Arc` references to it exist.
    pub fn drop(&self) {
        let ptr = self.private.ptr.swap(0, Ordering::SeqCst);
        if ptr != 0 {
            unsafe {
                mem::drop(Arc::from_raw(ptr as *const Font))
            }
        }
    }
}

unsafe fn clone_raw_arc<T: Send + Sync>(ptr: usize) -> Arc<T> {
    Arc::clone(&*mem::ManuallyDrop::new(Arc::from_raw(ptr as *const T)))
}
