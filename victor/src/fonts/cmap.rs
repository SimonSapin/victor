use fonts::{FontError, GlyphId};
use fonts::parsing::{Position, Slice, binary_search};
use fonts::tables::*;
use std::char;
use std::cmp::Ordering;
use std::mem;

pub(in fonts) enum Cmap {
    Format4(Format4),
    Format12(Format12),
}

impl Cmap {
    pub(in fonts) fn parse(bytes: &[u8], table_directory: Slice<TableDirectoryEntry>)
                            -> Result<Self, FontError> {
        let cmap_header = table_directory.find_table::<CmapHeader>(bytes)?;
        let cmap_records = Slice::new(
            cmap_header.followed_by::<CmapEncodingRecord>(),
            cmap_header.num_tables().read_from(bytes)?,
        );
        // Entries are sorted by (platform, encoding).
        // Iterator in reverse order to prefer (3, 10) over (3, 1).
        for record in cmap_records.into_iter().rev() {
            let subtable = cmap_header.offset_bytes(record.subtable_offset().read_from(bytes)?);
            let format = subtable.read_from(bytes)?;
            const MICROSOFT: u16 = 3;
            const UNICODE_USC2: u16 = 1;
            const UNICODE_USC4: u16 = 10;
            const SEGMENT_MAPPING_TO_DELTA_VALUES: u16 = 4;
            const SEGMENTED_COVERAGE: u16 = 12;
            match (
                record.platform_id().read_from(bytes)?,
                record.encoding_id().read_from(bytes)?,
                format,
            ) {
                (MICROSOFT, UNICODE_USC2, SEGMENT_MAPPING_TO_DELTA_VALUES) => {
                    return Ok(Cmap::Format4(Format4::parse(bytes, subtable.cast())?))
                }
                (MICROSOFT, UNICODE_USC4, SEGMENTED_COVERAGE) => {
                    return Ok(Cmap::Format12(Format12::parse(bytes, subtable.cast())?))
                }
                _ => {},
            }
        }

        Err(FontError::NoSupportedCmap)
    }

    pub(in fonts) fn each_code_point<F>(&self, bytes: &[u8], mut f: F)-> Result<(), FontError>
        where F: FnMut(char, GlyphId)
    {
        let f = move |code_point, glyph_id| {
            if glyph_id != 0 {
                // Ignore any mapping for surrogate code points
                if let Some(ch) = char::from_u32(code_point) {
                    f(ch, GlyphId(glyph_id));
                }
            }
        };
        match *self {
            Cmap::Format4(ref table) => table.each_code_point(bytes, f),
            Cmap::Format12(ref table) => table.each_code_point(bytes, f),
        }
    }
}

pub(in fonts) struct Format4 {
    end_codes: Slice<u16>,
    start_codes: Slice<u16>,
    // id_delta is really i16, but only used modulo 2^16 with u16::wrapping_add
    id_deltas: Slice<u16>,
    id_range_offsets: Slice<u16>,
}

impl Format4 {
    fn parse(bytes: &[u8], encoding_header: Position<CmapFormat4Header>) -> Result<Self, FontError> {
        let segment_count = encoding_header.segment_count_x2().read_from(bytes)? / 2;

        let end_codes = Slice::new(encoding_header.followed_by(), segment_count);
        let start_codes = Slice::new(
            end_codes
                .followed_by::<u16>() // "reservedPad"
                .followed_by(),
            segment_count,
        );
        let id_deltas = Slice::new(start_codes.followed_by(), segment_count);
        let id_range_offsets = Slice::new(id_deltas.followed_by(), segment_count);

        Ok(Format4 { end_codes, start_codes, id_deltas, id_range_offsets })
    }

    pub(in fonts) fn get(&self, bytes: &[u8], code_point: u32) -> Result<Option<u16>, FontError> {
        if code_point > 0xFFFF {
            return Ok(None)
        }
        let code_point = code_point as u16;

        let binary_search_result = binary_search(self.start_codes.count(), |segment_index| {
            if code_point > self.end_codes.get_unchecked(segment_index).read_from(bytes)? {
                Ok(Ordering::Less)
            } else if code_point < self.start_codes.get_unchecked(segment_index).read_from(bytes)? {
                Ok(Ordering::Greater)
            } else {
                Ok(Ordering::Equal)
            }
        });
        if let Some(segment_index) = binary_search_result? {
            let start_code = self.start_codes.get_unchecked(segment_index).read_from(bytes)?;
            self.glyph_id(bytes, segment_index, start_code, code_point)
        } else {
            Ok(None)
        }
    }

    fn each_code_point<F>(&self, bytes: &[u8], mut f: F) -> Result<(), FontError>
        where F: FnMut(u32, u16)
    {
        let mut segment_index = 0;
        for (end_code, start_code) in self.end_codes.into_iter().zip(self.start_codes) {
            let start_code = start_code.read_from(bytes)?;
            let end_code = end_code.read_from(bytes)?;
            let mut code_point = start_code;
            loop {
                if let Some(glyph_id) = self.glyph_id(bytes, segment_index, start_code, code_point)? {
                    f(u32::from(code_point), glyph_id)
                }

                if code_point == end_code {
                    break
                }
                code_point += 1
            }
            segment_index += 1
        }
        Ok(())
    }

    fn glyph_id(&self, bytes: &[u8], segment_index: u32, start_code: u16, code_point: u16)
                -> Result<Option<u16>, FontError> {
        let id_delta = self.id_deltas.get_unchecked(segment_index).read_from(bytes)?;
        let id_range_offset = self.id_range_offsets.get_unchecked(segment_index).read_from(bytes)?;

        let glyph_id = if id_range_offset != 0 {
            let offset = u32::from(id_range_offset) + mem::size_of::<u16>() as u32 * (
                segment_index + (code_point - start_code) as u32
            );
            let result: u16 = self.id_range_offsets.start().offset_bytes(offset).read_from(bytes)?;
            if result != 0 {
                result.wrapping_add(id_delta)
            } else {
                0
            }
        } else {
            code_point.wrapping_add(id_delta)
        };
        Ok(if glyph_id != 0 {
            Some(glyph_id)
        } else {
            None
        })
    }
}

pub(in fonts) struct Format12 {
    groups: Slice<CmapFormat12Group>,
}

impl Format12 {
    fn parse(bytes: &[u8], encoding_header: Position<CmapFormat12Header>) -> Result<Self, FontError> {
        Ok(Format12 {
            groups: Slice::new(
                encoding_header.followed_by(),
                encoding_header.num_groups().read_from(bytes)?,
            ),
        })
    }

    pub(in fonts) fn get(&self, bytes: &[u8], code_point: u32) -> Result<Option<u16>, FontError> {
        let result = binary_search(self.groups.count(), |index| {
            let group = self.groups.get_unchecked(index);
            if code_point < group.start_char_code().read_from(bytes)? {
                Ok(Ordering::Greater)
            } else if code_point > group.end_char_code().read_from(bytes)? {
                Ok(Ordering::Less)
            } else {
                Ok(Ordering::Equal)
            }
        });

        if let Some(index) = result? {
            let group = self.groups.get_unchecked(index);
            let id32 = group.start_glyph_id().read_from(bytes)? + (
                code_point - group.start_char_code().read_from(bytes)?
            );
            // Glyph IDs are 16 bits in PDF.
            // For now, pretend that glyphs with larger IDs are missing.
            // FIXME: Maybe this will be unnecessary with PDF font subsetting?
            if id32 <= 0xFFFF {
                return Ok(Some(id32 as u16))
            }
        }

        Ok(None)
    }

    fn each_code_point<F>(&self, bytes: &[u8], mut f: F) -> Result<(), FontError>
        where F: FnMut(u32, u16)
    {
        for group in self.groups {
            let start_code = group.start_char_code().read_from(bytes)?;
            let end_code = group.end_char_code().read_from(bytes)?;
            let start_glyph_id = group.start_glyph_id().read_from(bytes)?;
            let mut code_point = start_code;
            loop {
                let glyph_id = (code_point - start_code) + start_glyph_id;
                f(code_point, glyph_id as u16);

                if code_point == end_code {
                    break
                }
                code_point += 1;
            }
        }
        Ok(())
    }
}
