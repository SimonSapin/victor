use opentype;
use opentype::truetype::{FontHeader, CharMapping, NamingTable, HorizontalHeader, HorizontalMetrics};
use opentype::truetype::MaximumProfile;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, Cursor};
use std::mem;
use std::sync::Arc;

mod static_;

pub use self::static_::*;

pub struct Font {
    pub(crate) bytes: Cow<'static, [u8]>,
    pub(crate) postscript_name: String,
    pub(crate) cmap: HashMap<u16, u16>,
    pub(crate) min_x: i32,
    pub(crate) min_y: i32,
    pub(crate) max_x: i32,
    pub(crate) max_y: i32,
    pub(crate) ascent: i32,
    pub(crate) descent: i32,
    pub(crate) glyph_widths: Vec<u16>,
}

impl Font {
    pub fn from_bytes(bytes: Vec<u8>) -> io::Result<Arc<Self>> {
        Self::from_cow(bytes.into())
    }

    fn from_static(bytes: &'static [u8]) -> io::Result<Arc<Self>> {
        Self::from_cow(bytes.into())
    }

    fn from_cow(bytes: Cow<'static, [u8]>) -> io::Result<Arc<Self>> {
        let ot = opentype::Font::read(&mut Cursor::new(&*bytes))?;

        let version = ot.offset_table.header.version;
        const TRUETYPE: u32 = 0x74727565;  // "true" in big-endian
        if version != TRUETYPE && version != 0x_0001_0000 {
            Err(invalid("only TrueType fonts are supported"))?
        }

        macro_rules! take {
            () => {
                take!(())
            };
            ( $parameter: expr ) => {
                ot.take_given(&mut Cursor::new(&*bytes), $parameter)?
                  .ok_or_else(|| invalid("missing cmap table"))?
            }
        }

        let mut strings = match take!() {
            NamingTable::Format0(t) => t.strings(),
            NamingTable::Format1(t) => t.strings(),
        }?;

        // https://www.microsoft.com/typography/otspec/name.htm#nameIDs
        const POSTSCRIPT_NAME__NAME_ID: usize = 6;
        let postscript_name = mem::replace(&mut strings[POSTSCRIPT_NAME__NAME_ID], String::new());

        let cmaps: CharMapping = take!();
        let cmap = cmaps.encodings.iter()
            .map(|e| e.mapping())
            .filter(|m| !m.is_empty())
            .next()
            .ok_or_else(|| invalid("no supported cmap"))?;

        let header: FontHeader = take!();
        let horizontal_header: HorizontalHeader = take!();
        let ttf_units_per_em = i32::from(header.units_per_em);
        const PDF_GLYPH_SPACE_UNITS_PER_EM: i32 = 1000;
        let ttf_to_pdf = |x: i16| i32::from(x) * PDF_GLYPH_SPACE_UNITS_PER_EM / ttf_units_per_em;

        let maximum_profile: MaximumProfile = take!();
        let horizontal_metrics: HorizontalMetrics = take!((&horizontal_header, &maximum_profile));
        let last_glyph_id = cmap.values().cloned().max().unwrap();
        let glyph_widths = (0..last_glyph_id).map(|id| {
            let (width, _) = horizontal_metrics.get(id as usize);
            ttf_to_pdf(width as i16) as u16
        }).collect();

        Ok(Arc::new(Font {
            min_x: ttf_to_pdf(header.min_x),
            min_y: ttf_to_pdf(header.min_y),
            max_x: ttf_to_pdf(header.max_x),
            max_y: ttf_to_pdf(header.max_y),
            ascent: ttf_to_pdf(horizontal_header.ascender),
            descent: ttf_to_pdf(horizontal_header.descender),
            bytes, postscript_name, cmap, glyph_widths,
        }))
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
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
