use euclid;
use std::fmt::{self, Write};

/// The EM square unit
pub(in fonts2) struct Em;

/// The unit of FWord and UFWord
pub(in fonts2) struct FontDesignUnit;

pub(in fonts2) type FontDesignUnitsPerEmFactorU16 = euclid::TypedScale<u16, Em, FontDesignUnit>;

pub(in fonts2) type FWord = euclid::Length<i16, FontDesignUnit>;
pub(in fonts2) type UFWord = euclid::Length<u16, FontDesignUnit>;

/// 32-bit signed fixed-point number (16.16)
#[derive(Debug, Copy, Clone)]
pub(in fonts2) struct FixedPoint(pub u32);

/// Instant in time as seconds since 1904-01-01 midnight UTC
#[derive(Debug, Copy, Clone)]
pub(in fonts2) struct LongDateTime(pub i64);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ReadFromBytes)]
pub(in fonts2) struct Tag(pub [u8; 4]);


// ~~~~ Trait impls ~~~~

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &b in &self.0 {
            // ASCII printable or space
            f.write_char(if b' ' <= b && b <= b'~' { b } else { b'?' } as char)?
        }
        Ok(())
    }
}

impl From<LongDateTime> for ::std::time::SystemTime {
    fn from(instant: LongDateTime) -> Self {
        use std::time::{Duration, UNIX_EPOCH};

        // `date --utc -d 1904-01-01 +%s`
        let truetype_epoch = UNIX_EPOCH - Duration::from_secs(2_082_844_800);

        let seconds_since_truetype_epoch = instant.0;
        if seconds_since_truetype_epoch >= 0 {
            truetype_epoch + Duration::from_secs(seconds_since_truetype_epoch as u64)
        } else {
            truetype_epoch - Duration::from_secs((-seconds_since_truetype_epoch) as u64)
        }
    }
}
