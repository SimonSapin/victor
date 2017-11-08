use std::io;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem;
use super::Font;

/// Include a font as `static` data
///
/// This takes a filename relative to the macro’s use site source file (like `include_bytes!`)
/// and generates a function like this:
///
/// ```rust
/// pub fn $name() -> LazyStaticFontRef { … }
/// ```
///
/// Calling this function repeatedly will return objects
/// that reference the same static memory and singleton.
#[macro_export]
macro_rules! include_font {
    ($name: ident = $filename: expr) => {
        pub fn $name() -> $crate::fonts::LazyStaticFontRef {
            use std::sync::Mutex;
            use std::sync::atomic::{ATOMIC_USIZE_INIT, AtomicUsize};
            use $crate::fonts::LazyStaticFontRef;

            static BYTES: &'static [u8] = include_bytes!($filename);

            // There’s no ATOMIC_PTR_INIT :(
            static FONT_PTR: AtomicUsize = ATOMIC_USIZE_INIT;

            lazy_static! {
                static ref MUTEX: Mutex<()> = Mutex::new(());
            }

            unsafe {
                LazyStaticFontRef::from_statics(BYTES, &FONT_PTR, &*MUTEX)
            }
        }
    }
}

/// The regular sans-serif face of the [Bitstream Vera](https://www.gnome.org/fonts/) font family.
include_font!(bitstream_vera_sans = "../../fonts/vera/Vera.ttf");

/// A reference to a lazily-parsed font backed by a static bytes slice.
pub struct LazyStaticFontRef {
    bytes: &'static [u8],
    ptr: &'static AtomicUsize,
    mutex: &'static Mutex<()>,
}

impl LazyStaticFontRef {
    /// Unsafe because `ptr` must be initially zero
    /// and must not be accessed outside of a `LazyStaticFontRef`.
    #[doc(hidden)]
    pub unsafe fn from_statics(
        bytes: &'static [u8],
        ptr: &'static AtomicUsize,
        mutex: &'static Mutex<()>,
    ) -> Self {
        LazyStaticFontRef { bytes, ptr, mutex }
    }

    /// Return this font’s raw data
    pub fn bytes(&self) -> &'static [u8] {
        self.bytes
    }

    // FIXME: figure out minimal Ordering for atomic accesses

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
                let ptr = self.ptr.load(Ordering::SeqCst);
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
        let _guard = self.mutex.lock();

        // Try again in case some other thread raced us while we were taking the mutex
        try_load!();

        // Now we’ve observed the atomic pointer uninitialized after taking the mutex:
        // we’re definitely first

        let font = Font::from_bytes(self.bytes)?;
        let new_ptr = Arc::into_raw(font.clone()) as usize;
        self.ptr.store(new_ptr, Ordering::SeqCst);
        Ok(font)
    }

    /// Deinitialize this font’s singleton, dropping the internal `Arc` reference.
    ///
    /// Calling `.get()` again afterwards will parse a new `Font` object.
    ///
    /// The previous `Font` object may continue to live as long
    /// as other `Arc` references to it exist.
    pub fn drop(&self) {
        let ptr = self.ptr.swap(0, Ordering::SeqCst);
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
