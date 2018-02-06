use fonts::{Font, FontError, GlyphId, Em};
use primitives::Length;
use std::sync::Arc;
use xi_unicode::LineBreakIterator;

pub struct ShapedSegment {
    pub(crate) font: Arc<Font>,
    pub(crate) glyphs: Vec<GlyphId>,
    pub(crate) advance_width: Length<Em>,
}

impl ShapedSegment {
    /// Simplistic text shaping:
    ///
    /// * No font fallback
    /// * No support for complex scripts
    /// * No ligatures
    /// * No kerning
    pub fn naive_shape(text: &str, font: Arc<Font>) -> Result<Self, FontError> {
        let mut glyphs = Vec::new();
        let mut advance_width = Length::new(0.);
        for ch in text.chars() {
            let id = font.glyph_id(ch)?;
            advance_width += font.glyph_width(id)?;
            glyphs.push(id);
        }
        Ok(ShapedSegment { font, glyphs, advance_width })
    }
}

pub fn split_at_breaks(s: &str) -> Vec<&str> {
    let mut last_break = 0;
    LineBreakIterator::new(s).map(|(position, _)| {
        let range = last_break..position;
        last_break = position;
        &s[range]
    }).collect()
}

pub fn split_at_hard_breaks(s: &str) -> Vec<&str> {
    let mut last_break = 0;
    LineBreakIterator::new(s).filter(|&(_, is_hard_break)| is_hard_break).map(|(position, _)| {
        let range = last_break..position;
        last_break = position;
        &s[range]
    }).collect()
}
