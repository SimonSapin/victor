use crate::fonts::{Em, Font, FontError, GlyphId};
use crate::primitives::Length;
use std::sync::Arc;

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
        Ok(ShapedSegment {
            font,
            glyphs,
            advance_width,
        })
    }
}
