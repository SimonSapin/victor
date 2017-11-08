use std::io;
use std::sync::{Arc, Once, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem;
use super::Font;

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
            _private_unsafe_ptr: ::std::sync::atomic::ATOMIC_USIZE_INIT,
            _private_unsafe_mutex: ::std::sync::atomic::ATOMIC_USIZE_INIT,
            _private_unsafe_mutex_init: ::std::sync::ONCE_INIT,
        }
    }
}

/// The regular sans-serif face of the [Bitstream Vera](https://www.gnome.org/fonts/) font family.
pub static BITSTREAM_VERA_SANS: LazyStaticFont = include_font!("../../fonts/vera/Vera.ttf");

/// A lazily-parsed font backed by a static bytes slice.
pub struct LazyStaticFont {
    /// This font’s raw data
    pub bytes: &'static [u8],

    #[doc(hidden)] pub _private_unsafe_ptr: AtomicUsize,
    #[doc(hidden)] pub _private_unsafe_mutex_init: Once,

    // This doesn’t need to be atomic (it’s already synchronized with `mutex_init`)
    // but we can’t construct `UnsafeCell<_>` in a static on the stable channel
    #[doc(hidden)] pub _private_unsafe_mutex: AtomicUsize,
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
    pub fn get(&'static self) -> io::Result<Arc<Font>> {
        macro_rules! try_load {
            () => {
                let ptr = self._private_unsafe_ptr.load(Ordering::SeqCst);
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
        self._private_unsafe_mutex_init.call_once(|| {
            let ptr: *const Mutex<()> = Box::into_raw(Box::new(Mutex::new(())));
            self._private_unsafe_mutex.store(ptr as usize, Ordering::Relaxed);
        });
        let ptr = self._private_unsafe_mutex.load(Ordering::Relaxed) as *const Mutex<()>;
        let mutex = unsafe { &*ptr };
        let guard = mutex.lock();

        // Try again in case some other thread raced us while we were taking the mutex
        try_load!();

        // Now we’ve observed the atomic pointer uninitialized after taking the mutex:
        // we’re definitely first

        let font = Font::from_bytes(self.bytes)?;
        let new_ptr = Arc::into_raw(font.clone()) as usize;
        self._private_unsafe_ptr.store(new_ptr, Ordering::SeqCst);

        mem::drop(guard);
        Ok(font)
    }

    /// Deinitialize this font’s singleton, dropping the internal `Arc` reference.
    ///
    /// Calling `.get()` again afterwards will parse a new `Font` object.
    ///
    /// The previous `Font` object may continue to live as long
    /// as other `Arc` references to it exist.
    pub fn drop(&self) {
        let ptr = self._private_unsafe_ptr.swap(0, Ordering::SeqCst);
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
