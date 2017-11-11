use std::char;
use std::cmp::Ordering;
use std::mem::size_of;
use super::{FontError, GlyphId};
use super::ttf_tables::{CmapFormat4Header, CmapFormat12Header, CmapFormat12Group};
use super::ttf_types::{Pod, AlignedBytes, u16_be};

pub(crate) enum Cmap {
    Format4 { offset: usize },
    Format12 { offset: usize },
}

impl Cmap {
    pub(crate) fn each_code_point<F>(&self, bytes: AlignedBytes, mut f: F)-> Result<(), FontError>
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
            Cmap::Format4 { offset } => Format4::parse(bytes, offset)?.each_code_point(f),
            Cmap::Format12 { offset } => Ok(Format12::parse(bytes, offset)?.each_code_point(f)),
        }
    }
}

pub(crate) struct Format4<'bytes> {
    bytes: AlignedBytes<'bytes>,
    end_codes: &'bytes [u16_be],
    start_codes: &'bytes [u16_be],
    id_deltas: &'bytes [u16_be],
    id_range_offsets: &'bytes [u16_be],
    id_range_offsets_start: usize,
}

impl<'bytes> Format4<'bytes> {
    pub(crate) fn parse(bytes: AlignedBytes<'bytes>, record_offset: usize)
                        -> Result<Self, FontError> {
        let encoding_header = CmapFormat4Header::cast(bytes, record_offset)?;
        let segment_count = encoding_header.segment_count_x2.value() as usize / 2;
        let subtable_size = segment_count.saturating_mul(size_of::<u16>());

        let end_codes_start = record_offset
            .saturating_add(size_of::<CmapFormat4Header>());
        let start_codes_start = end_codes_start
            .saturating_add(subtable_size)  // Add end_code subtable
            .saturating_add(size_of::<u16>());  // Add reserved_padding
        let id_deltas_start = start_codes_start.saturating_add(subtable_size);
        let id_range_offsets_start = id_deltas_start.saturating_add(subtable_size);

        Ok(Format4 {
            bytes,
            // id_delta is really i16, but only used modulo 2^16 with u16::wrapping_add
            end_codes: u16_be::cast_slice(bytes, end_codes_start, segment_count)?,
            start_codes: u16_be::cast_slice(bytes, start_codes_start, segment_count)?,
            id_deltas: u16_be::cast_slice(bytes, id_deltas_start, segment_count)?,
            id_range_offsets: u16_be::cast_slice(bytes, id_range_offsets_start, segment_count)?,
            id_range_offsets_start,
        })
    }

    pub(crate) fn get(&self, code_point: u32) -> Result<Option<u16>, FontError> {
        if code_point > 0xFFFF {
            return Ok(None)
        }
        let code_point = code_point as u16;

        // This a modification of [T]::binary_search_by
        // that passes the current index to the closure,
        // so we can use it with Format4’s parallel slices.
        fn binary_search_by<'a, T, F>(mut s: &'a [T], mut f: F) -> Result<usize, usize>
            where F: FnMut(usize, &'a T) -> Ordering
        {
            let mut base = 0;

            loop {
                let (head, tail) = s.split_at(s.len() >> 1);
                if tail.is_empty() {
                    return Err(base)
                }
                let index = base + head.len();
                match f(index, &tail[0]) {
                    Ordering::Less => {
                        base = index + 1;
                        s = &tail[1..];
                    }
                    Ordering::Greater => s = head,
                    Ordering::Equal => return Ok(index),
                }
            }
        }
        let binary_search_result = binary_search_by(self.end_codes, |segment_index, end_code| {
            if code_point > end_code.value() {
                Ordering::Less
            } else if code_point < self.start_codes[segment_index].value() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        if let Ok(segment_index) = binary_search_result {
            let start_code = self.start_codes[segment_index].value();
            self.glyph_id(segment_index, start_code, code_point)
        } else {
            Ok(None)
        }
    }

    fn each_code_point<F>(&self, mut f: F) -> Result<(), FontError> where F: FnMut(u32, u16) {
        let iter = self.end_codes.iter().zip(self.start_codes);
        for (segment_index, (end_code, start_code)) in iter.enumerate() {
            let start_code = start_code.value();
            let end_code = end_code.value();
            let mut code_point = start_code;
            loop {
                if let Some(glyph_id) = self.glyph_id(segment_index, start_code, code_point)? {
                    f(u32::from(code_point), glyph_id)
                }

                if code_point == end_code {
                    break
                }
                code_point += 1;
            }
        }
        Ok(())
    }

    fn glyph_id(&self, segment_index: usize, start_code: u16, code_point: u16)
                -> Result<Option<u16>, FontError> {
        let id_delta = self.id_deltas[segment_index].value();
        let id_range_offset = self.id_range_offsets[segment_index].value();

        let glyph_id = if id_range_offset != 0 {
            let offset =
                self.id_range_offsets_start +
                segment_index * size_of::<u16>() +
                id_range_offset as usize +
                (code_point - start_code) as usize * size_of::<u16>();
            let result = u16_be::cast(self.bytes, offset)?.value();
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

pub(crate) struct Format12<'bytes> {
    groups: &'bytes [CmapFormat12Group],
}

impl<'bytes> Format12<'bytes> {
    pub(crate) fn parse(bytes: AlignedBytes<'bytes>, record_offset: usize)
                        -> Result<Self, FontError> {
        let encoding_header = CmapFormat12Header::cast(bytes, record_offset)?;
        let groups = CmapFormat12Group::cast_slice(bytes,
            record_offset.saturating_add(size_of::<CmapFormat12Header>()),
            encoding_header.num_groups.value() as usize,
        )?;
        Ok(Format12 { groups })
    }

    pub(crate) fn get(&self, code_point: u32) -> Option<u16> {
        self.groups.binary_search_by(|group| {
            if code_point < group.start_char_code.value() {
                Ordering::Greater
            } else if code_point > group.end_char_code.value() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }).ok().map(|index| {
            let group = &self.groups[index];
            ((code_point - group.start_char_code.value()) + group.start_glyph_id.value()) as u16
        })
    }

    fn each_code_point<F>(&self, mut f: F) where F: FnMut(u32, u16) {
        for group in self.groups {
            let start_code = group.start_char_code.value();
            let end_code = group.end_char_code.value();
            let start_glyph_id = group.start_glyph_id.value();
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
    }
}
