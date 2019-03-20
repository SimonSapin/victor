use crate::fonts::{Em, Font, FontError, GlyphId};
use crate::primitives::Length;
use std::sync::Arc;

#[derive(Clone)]
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
        let mut s = Self::new_with_naive_shaping(font);
        s.append(text.chars())?;
        Ok(s)
    }

    pub fn new_with_naive_shaping(font: Arc<Font>) -> Self {
        Self {
            font,
            glyphs: Vec::new(),
            advance_width: Length::new(0.),
        }
    }

    pub fn append(&mut self, text: impl Iterator<Item = char>) -> Result<(), FontError> {
        for ch in text {
            let id = self.font.glyph_id(ch)?;
            self.advance_width += self.font.glyph_width(id)?;
            self.glyphs.push(id);
        }
        Ok(())
    }
}
