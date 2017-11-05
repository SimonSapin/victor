use opentype::{self, Table};
use std::borrow::Cow;
use std::io::{self, Cursor};
use std::sync::Arc;
use truetype::FontHeader;

pub struct Font {
    ot: opentype::Font,
    bytes: Cow<'static, [u8]>,
}

impl Font {
    pub fn from_bytes<T: Into<Cow<'static, [u8]>>>(bytes: T) -> io::Result<Arc<Self>> {
        Self::from_cow(bytes.into())
    }

    fn from_cow(bytes: Cow<'static, [u8]>) -> io::Result<Arc<Self>> {
        let ot = opentype::Font::read(&mut Cursor::new(&*bytes))?;
        let version = ot.offset_table.header.version;
        const TRUETYPE: u32 = 0x74727565;  // "true" in big-endian
        if version != TRUETYPE && version != 0x00010000 {
            error("only TrueType fonts are supported")?
        }
        Ok(Arc::new(Font { ot, bytes }))
    }

    fn take<'a, T>(&self) -> io::Result<T> where T: Table<'a, Parameter=()> {
        self.take_given(())
    }

    fn take_given<'a, T>(&self, parameter: T::Parameter) -> io::Result<T> where T: Table<'a> {
        self.ot.take_given(&mut Cursor::new(&*self.bytes), parameter)?
        .ok_or_else(|| invalid("missing font table"))
    }

    pub fn units_per_em(&self) -> io::Result<u16> {
        Ok(self.take::<FontHeader>()?.units_per_em)
    }
}

fn error(message: &str) -> io::Result<()> {
    Err(invalid(message))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
