// Copyright 2017 Simon Sapin
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option.

//! Similar to https://crates.io/crates/lazy_static but:
//!
//! * The static value can be “deinitialized” (dropped).
//!   `Arc` is used to do so safely without invalidating existing references.
//! * Initialization can return an error (for example if it involves parsing).
//!
//! # Example
//!
//! ```rust
//! static FOO: LazyArc<Foo> = LazyArc::INIT;
//!
//! let foo = FOO.get_or_create(|| Ok(Arc::new(include_str!("something").parse()?))?;
//! ```

use lock_api::RawMutex as RawMutexTrait;
use parking_lot::RawMutex;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;

pub struct LazyArc<T: Send + Sync> {
    poltergeist: PhantomData<Arc<T>>,
    mutex: RawMutex,
    ptr: AtomicPtr<T>,
}

impl<T: Send + Sync> LazyArc<T> {
    pub const INIT: Self = LazyArc {
        poltergeist: PhantomData,
        mutex: RawMutex::INIT,
        ptr: AtomicPtr::new(ptr::null_mut()),
    };

    // FIXME: figure out minimal Ordering for atomic operations

    /// Return a new `Arc` reference to the singleton `T` object.
    ///
    /// If this singleton was not already initialized,
    /// try to call the closure now (this may return an error) to initialize it.
    ///
    /// Calling this reapeatedly will only initialize once (until `.drop()` is called).
    pub fn get_or_create<F, E>(&self, create: F) -> Result<Arc<T>, E>
    where
        F: FnOnce() -> Result<Arc<T>, E>,
    {
        macro_rules! try_load {
            () => {
                let ptr = self.ptr.load(Ordering::SeqCst);
                if !ptr.is_null() {
                    // Already initialized

                    // We want to create a new strong reference (with `clone()`)
                    // but not drop the existing one.
                    // `Arc::from_raw` normally takes ownership of a strong reference,
                    // so use `ManuallyDrop` to skip running that destructor.
                    let careful_dont_drop_it = ManuallyDrop::new(unsafe { Arc::from_raw(ptr) });
                    return Ok(Arc::clone(&*careful_dont_drop_it))
                }
            };
        }

        // First try to obtain an Arc from the atomic pointer without taking the mutex
        try_load!();

        // Synchronize initialization
        struct RawMutexGuard<'a>(&'a RawMutex);
        impl<'a> Drop for RawMutexGuard<'a> {
            fn drop(&mut self) {
                self.0.unlock()
            }
        }

        self.mutex.lock();
        let _guard = RawMutexGuard(&self.mutex);

        // Try again in case some other thread raced us while we were taking the mutex
        try_load!();

        // Now we’ve observed the atomic pointer uninitialized after taking the mutex:
        // we’re definitely first

        let data = create()?;
        let new_ptr = Arc::into_raw(data.clone()) as *mut T;
        self.ptr.store(new_ptr, Ordering::SeqCst);
        Ok(data)
    }

    // Oops, this turned out to be unsound:
    // If drop() is called while another thread is in them middle of get_or_create()
    // after self.ptr.load() but before Arc::clone(),
    // the refcount could drop to zero and the arc be deallocated,
    // causing a use-after-free in the other thread.

    //    /// Deinitialize this singleton, dropping the internal `Arc` reference.
    //    ///
    //    /// Calling `.get()` again afterwards will create a new `T` object.
    //    ///
    //    /// The previous `T` object may continue to live as long
    //    /// as other `Arc` references to it exist.
    //    pub fn drop(&self) {
    //        let ptr = self.ptr.swap(ptr::null_mut(), Ordering::SeqCst);
    //        if !ptr.is_null() {
    //            unsafe {
    //                mem::drop(Arc::from_raw(ptr))
    //            }
    //        }
    //    }
}
