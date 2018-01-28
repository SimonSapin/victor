use std::fmt::{self, Write};

pub(in fonts2) type FixedPoint = u32;

#[derive(PartialEq, Eq, PartialOrd, Ord, ReadFromBytes)]
pub(in fonts2) struct Tag(pub [u8; 4]);

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &b in &self.0 {
            // ASCII printable or space
            f.write_char(if b' ' <= b && b <= b'~' { b } else { b'?' } as char)?
        }
        Ok(())
    }
}
