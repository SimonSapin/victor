use opentype::{self, Table};
use opentype::truetype::{CharMapping, NamingTable};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Cursor};
use std::mem;
use std::sync::Arc;

pub struct Font {
    ot: opentype::Font,
    cmap: HashMap<u16, u16>,
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
        if version != TRUETYPE && version != 0x_0001_0000 {
            error("only TrueType fonts are supported")?
        }

        let cmaps: CharMapping = ot
            .take(&mut Cursor::new(&*bytes))?
            .ok_or_else(|| invalid("missing cmap table"))?;
        let cmap = cmaps.encodings.iter()
            .map(|e| e.mapping())
            .filter(|m| !m.is_empty())
            .next()
            .ok_or_else(|| invalid("no supported cmap"))?;

        Ok(Arc::new(Font { ot, cmap, bytes }))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_glyph_ids(&self, text: &str) -> Vec<u16> {
        text.chars().map(|c| {
            let c = c as u32;
            const NOTDEF_GLYPH: u16 = 0;
            if c <= 0xFFFF {
                self.cmap.get(&(c as u16)).cloned().unwrap_or(NOTDEF_GLYPH)
            } else {
                NOTDEF_GLYPH
            }
        }).collect()
    }

    fn take<'a, T>(&self) -> io::Result<T> where T: Table<'a, Parameter=()> {
        self.take_given(())
    }

    fn take_given<'a, T>(&self, parameter: T::Parameter) -> io::Result<T> where T: Table<'a> {
        self.ot.take_given(&mut Cursor::new(&*self.bytes), parameter)?
        .ok_or_else(|| invalid("missing font table"))
    }

    fn naming_strings(&self) -> io::Result<Vec<String>> {
        match self.take()? {
            NamingTable::Format0(t) => t.strings(),
            NamingTable::Format1(t) => t.strings(),
        }
    }

    pub fn postscript_name(&self) -> String {
        let mut strings = self.naming_strings().unwrap();
        // https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6name.html
        const POSTSCRIPT_NAME__NAME_ID: usize = 6;
        mem::replace(&mut strings[POSTSCRIPT_NAME__NAME_ID], String::new())
    }
}

fn error(message: &str) -> io::Result<()> {
    Err(invalid(message))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
