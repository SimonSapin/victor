use std::borrow::Cow;
use std::fmt::{self, Write};
use std::mem;
use std::slice;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use super::FontError;

macro_rules! big_endian_int_wrappers {
    ($( $Name: ident: $Int: ty; )+) => {
        $(
            /// A big-endian integer
            #[derive(Pod)]
            #[repr(C)]
            pub(crate) struct $Name($Int);

            impl $Name {
                /// Return the value in native-endian
                #[inline]
                pub(crate) fn value(&self) -> $Int {
                    <$Int>::from_be(self.0)
                }
            }
        )+
    }
}

big_endian_int_wrappers! {
    u32_be: u32;
    u16_be: u16;
    i16_be: i16;
}

pub(crate) type FWord = i16_be;
pub(crate) type UFWord = u16_be;

/// `Fixed` in https://www.microsoft.com/typography/otspec/otff.htm#dataTypes
#[repr(C)]
#[derive(Pod)]
pub(crate) struct FixedPoint {
    integral: u16_be,
    fractional: u16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct LongDateTime {
    // These two field represent a single i64.
    // We split it in two because TrueType only requires 4-byte alignment.
    pub upper_bits: u32_be,
    pub lower_bits: u32_be,
}

#[derive(Pod)]
#[repr(C)]
pub(crate) struct Tag(pub [u8; 4]);

impl LongDateTime {
    fn seconds_since_1904_01_01_midnight(&self) -> i64 {
        let upper = (self.upper_bits.value() as u64) << 32;
        let lower = self.lower_bits.value() as u64;
        (upper | lower) as i64
    }

    fn to_system_time(&self) -> SystemTime {
        // `date --utc -d 1904-01-01 +%s`
        let truetype_epoch = UNIX_EPOCH - Duration::from_secs(2_082_844_800);
        let seconds = self.seconds_since_1904_01_01_midnight();
        if seconds >= 0 {
            truetype_epoch + Duration::from_secs(seconds as u64)
        } else {
            truetype_epoch - Duration::from_secs((-seconds) as u64)
        }
    }
}

impl fmt::Debug for LongDateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_system_time().fmt(f)
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_char('"')?;
        for &byte in &self.0 {
            if byte == b'"' {
                f.write_str(r#"\""#)?
            } else if b' ' <= byte && byte <= b'~' {
                // ASCII printable or space
                f.write_char(byte as char)?
            } else {
                write!(f, r"\x{:02X}", byte)?
            }
        }
        f.write_char('"')
    }
}

const TRUETYPE_TABLE_ALIGNMENT: usize = 4;

#[derive(Copy, Clone)]
pub(crate) struct AlignedBytes<'a>(&'a [u8]);

pub(crate) struct AlignedCowBytes(pub(crate) Cow<'static, [u8]>);

impl AlignedCowBytes {
    pub fn new(bytes: Cow<'static, [u8]>) -> Option<Self> {
        let is_aligned = (bytes.as_ptr() as usize) % TRUETYPE_TABLE_ALIGNMENT == 0;
        // Allow empty slices here because we want Pod::cast to return an error
        // rather than Font::from_* to panic.
        if is_aligned || bytes.is_empty() {
            Some(AlignedCowBytes(bytes))
        } else {
            None
        }
    }

    pub fn empty() -> Self {
        AlignedCowBytes(Cow::Borrowed(&[]))
    }

    pub fn borrow(&self) -> AlignedBytes {
        AlignedBytes(&self.0)
    }
}

#[inline]
fn cast_ptr<T>(bytes: AlignedBytes, offset: usize, n_items: usize) -> Result<*const T, FontError> {
    // FIXME: could this be a static assert?
    let required_alignment = mem::align_of::<T>();
    assert!(required_alignment <= 4,
            "This type requires more alignment than TrueType promises");

    let required_len = mem::size_of::<T>().saturating_mul(n_items);
    let bytes = bytes.0.get(offset..).ok_or(FontError::OffsetBeyondEof)?;
    let bytes = bytes.get(..required_len).ok_or(FontError::OffsetPlusLengthBeyondEof)?;

    Ok(bytes.as_ptr() as *const T)
}

/// Plain old data: all bit patterns represent valid values
pub(crate) unsafe trait Pod: Sized {
    #[inline]
    fn cast(bytes: AlignedBytes, offset: usize) -> Result<&Self, FontError> {
        let ptr = cast_ptr(bytes, offset, 1)?;
        unsafe {
            Ok(&*ptr)
        }
    }

    #[inline]
    fn cast_slice(bytes: AlignedBytes, offset: usize, n_items: usize) -> Result<&[Self], FontError> {
        let ptr = cast_ptr(bytes, offset, n_items)?;
        unsafe {
            Ok(slice::from_raw_parts(ptr, n_items))
        }
    }
}

unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for u32 {}
unsafe impl<T: Pod> Pod for [T; 4] {}
