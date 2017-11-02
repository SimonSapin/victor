// FIXME: use std::convert::TryInto when it’s stable

// Poppler probably doesn’t run in a 64 KB address space anyway.
#![cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]

use std::fmt;

pub(crate) trait TryInto<T> {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}

pub(crate) struct TryIntoIntError(());
pub(crate) enum Infallible {}

impl fmt::Debug for TryIntoIntError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Out of range integer")
    }
}

impl fmt::Debug for Infallible {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        match *self {}
    }
}

// no possible bounds violation
macro_rules! try_into_unbounded {
    ($source:ty, $target:ty) => {
        impl TryInto<$target> for $source {
            type Error = Infallible;

            #[inline]
            fn try_into(self) -> Result<$target, Self::Error> {
                Ok(self as $target)
            }
        }
    }
}

// only negative bounds
macro_rules! try_into_lower_bounded {
    ($source:ty, $target:ty) => {
        impl TryInto<$target> for $source {
            type Error = TryIntoIntError;

            #[inline]
            fn try_into(self) -> Result<$target, TryIntoIntError> {
                if self >= 0 {
                    Ok(self as $target)
                } else {
                    Err(TryIntoIntError(()))
                }
            }
        }
    }
}

// unsigned to signed (only positive bound)
macro_rules! try_into_upper_bounded {
    ($source:ty, $target:ty) => {
        impl TryInto<$target> for $source {
            type Error = TryIntoIntError;

            #[inline]
            fn try_into(self) -> Result<$target, TryIntoIntError> {
                if self > (<$target>::max_value() as $source) {
                    Err(TryIntoIntError(()))
                } else {
                    Ok(self as $target)
                }
            }
        }
    }
}

// all other cases
macro_rules! try_into_both_bounded {
    ($source:ty, $target:ty) => {
        impl TryInto<$target> for $source {
            type Error = TryIntoIntError;

            #[inline]
            fn try_into(self) -> Result<$target, TryIntoIntError> {
                let min = <$target>::min_value() as $source;
                let max = <$target>::max_value() as $source;
                if self < min || self > max {
                    Err(TryIntoIntError(()))
                } else {
                    Ok(self as $target)
                }
            }
        }
    }
}

try_into_both_bounded!(f64, i32);
try_into_upper_bounded!(usize, i32);
try_into_lower_bounded!(i32, usize);
try_into_unbounded!(u32, usize);
