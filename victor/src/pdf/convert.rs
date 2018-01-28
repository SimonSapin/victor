use euclid;
use fonts2::{Font, GlyphId, FontError, Em, FontDesignUnit};
use pdf::object::{Object, Dictionary};
use pdf::syntax::{PdfFile, PAGE_TREE_ID, BasicObjects};
use primitives::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash;
use std::io::{self, Write};
use std::ops::Deref;
use std::sync::Arc;

const PT_PER_INCH: f32 = 72.;
const PX_PER_INCH: f32 = 96.;
const PT_PER_PX: f32 = PT_PER_INCH / PX_PER_INCH;
const CSS_TO_PDF_SCALE_X: f32 = PT_PER_PX;
const CSS_TO_PDF_SCALE_Y: f32 = -PT_PER_PX;  // Flip the Y axis direction, it defaults to upwards in PDF.

struct PdfGlyphSpace;

/// FIXME: Is this precisely defined somewhere?
type PdfTextSpace = Em;

/// https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G8.1695902
/// “for all font types except Type 3,
///  the units of glyph space are one-thousandth of a unit of text space”
fn glyph_space_units_per_em<T>() -> euclid::TypedScale<T, PdfTextSpace, PdfGlyphSpace>
    where T: ::num_traits::FromPrimitive
{
    euclid::TypedScale::new(T::from_u16(1000).unwrap())
}

pub(crate) struct InProgressDoc {
    pdf: PdfFile,
    page_ids: Vec<Object<'static>>,
    extended_graphics_states: Vec<(Vec<u8>, Object<'static>)>,
    font_resources: Vec<(Vec<u8>, Object<'static>)>,
    alpha_states: HashMap<u16, String>,
    fonts: HashMap<ByAddress<Arc<Font>>, String>,
}

impl InProgressDoc {
    pub(crate) fn new() -> Self {
        InProgressDoc {
            pdf: PdfFile::new(),
            page_ids: Vec::new(),
            extended_graphics_states: Vec::new(),
            font_resources: Vec::new(),
            alpha_states: HashMap::new(),
            fonts: HashMap::new(),
        }
    }

    pub(crate) fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.pdf.write(w, &BasicObjects {
            page_tree: dictionary! {
                "Type" => "Pages",
                "Count" => self.page_ids.len(),
                "Kids" => &*self.page_ids,
                "Resources" => dictionary! {
                    "Font" => Object::DictionaryWithOwnedKeys(&self.font_resources),
                    "ExtGState" => Object::DictionaryWithOwnedKeys(&self.extended_graphics_states),
                },
            },
            catalog: dictionary! {
                "Type" => "Catalog",
                "Pages" => PAGE_TREE_ID,
            },
            info: dictionary! {
                "Producer" => Object::LiteralString(b"Victor <https://github.com/SimonSapin/victor>"),
            },
        })
    }
}

struct ByAddress<T>(T);

impl<T> hash::Hash for ByAddress<T> where T: Deref, T::Target: Sized {
    fn hash<H>(&self, state: &mut H) where H: hash::Hasher {
        (self.0.deref() as *const T::Target as usize).hash(state)
    }
}

impl<T> PartialEq for ByAddress<T> where T: Deref, T::Target: Sized {
    fn eq(&self, other: &Self) -> bool {
        (self.0.deref() as *const T::Target as usize) ==
        (other.0.deref() as *const T::Target as usize)
    }
}

impl<T> Eq for ByAddress<T> where T: Deref, T::Target: Sized {}

pub(crate) struct InProgressPage<'a> {
    doc: &'a mut InProgressDoc,
    size: Size<CssPx>,
    operations: Vec<u8>,
    graphics_state: GraphicsState,
}

impl<'a> Drop for InProgressPage<'a> {
    fn drop(&mut self) {
        let content_id = self.doc.pdf.add_stream(
            dictionary! {},
            self.operations.as_slice().into()
        );
        let page_id = self.doc.pdf.add_dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => PAGE_TREE_ID,
            "Contents" => content_id,
            "MediaBox" => array![
                0,
                0,
                self.size.width * CSS_TO_PDF_SCALE_X,
                self.size.height * CSS_TO_PDF_SCALE_Y,
            ],
        });
        self.doc.page_ids.push(page_id.into());
    }
}

struct GraphicsState {
    non_stroking_color_rgb: (f32, f32, f32),
    alpha: f32,
}

macro_rules! op {
    ( $self_: expr, $operator: expr ) => {
        op!($self_, $operator,)
    };
    ( $self_: expr, $operator: expr, $( $operands: expr ),*) => {
        {
            $(
                Object::from($operands).write(&mut $self_.operations).unwrap();
                $self_.operations.push(b' ');
            )*
            $self_.operations.extend(str::as_bytes($operator));
            $self_.operations.push(b'\n');
        }
    }
}

impl<'a> InProgressPage<'a> {
    pub fn new(doc: &'a mut InProgressDoc, size: Size<CssPx>) -> Self {
        let mut page = InProgressPage {
            doc,
            size,
            operations: Vec::new(),
            // Initial state:
            graphics_state: GraphicsState {
                non_stroking_color_rgb: (0., 0., 0.),  // Black
                alpha: 1.,  // Fully opaque
            },
        };
        op!(page, CURRENT_TRANSFORMATION_MATRIX, CSS_TO_PDF_SCALE_X, 0, 0, CSS_TO_PDF_SCALE_Y, 0, 0);
        page
    }

    pub(crate) fn set_color(&mut self, &RGBA(r, g, b, a): &RGBA) {
        if self.graphics_state.non_stroking_color_rgb != (r, g, b) {
            self.graphics_state.non_stroking_color_rgb = (r, g, b);
            op!(self, NON_STROKING_RGB_COLOR, r, g, b);
        }
        self.set_alpha(a)
    }

    pub(crate) fn paint_rectangle(&mut self, rect: &Rect<CssPx>) {
        op!(self, RECTANGLE, rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
        op!(self, FILL);
    }

    pub(crate) fn show_text(&mut self, text: &TextRun) -> Result<(), FontError> {
        let TextRun { ref font, ref font_size, ref origin, ref glyph_ids } = *text;
        let font_key = self.add_font(font)?;
        // flip the Y axis in to compensate the same flip at the page level.
        let x_scale = font_size.0;
        let y_scale = -font_size.0;
        let mut glyph_codes = Vec::with_capacity(glyph_ids.len() * 2);
        for &GlyphId(id) in glyph_ids {
            // Big-endian
            glyph_codes.push((id >> 8) as u8);
            glyph_codes.push(id as u8);
        }
        op!(self, BEGIN_TEXT);
        op!(self, TEXT_FONT_AND_SIZE, &*font_key, 1);
        op!(self, TEXT_MATRIX, x_scale, 0, 0, y_scale, origin.x, origin.y);
        op!(self, SHOW_TEXT, Object::HexString(&glyph_codes));
        op!(self, END_TEXT);

        // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G8.1910927
        // In the Font resource dictionary:
        // subtype (TrueType), PS name, font program (stream), advance width of each glyph
        // glyph space: 1/1000 of text space
        // text space: controlled by text maxtrix (Tm op) and text size (Tf op) based on user space
        // Td op: place current text position (origin of text space) in user space
        // glyph displacement vector translates text space when showing a glyph, based on font metrics
        // writing mode: 0 is horizontal, 1 is vertical
        //      vertical: no “vhea” and “vmtx” tables, DW2 and W2 entries in a CIDFont dict
        // more than 1 byte per glyph ID: composite fonts
        // Embedded font stream dictionary: /Length1 decoded TrueType size
        // TrueType tables required:
        // “head”, “hhea”, “loca”, “maxp”, “cvt”, “prep”, “glyf”, “hmtx”, and “fpgm”
        // Subset: prefix /BaseFont name with 6 upper case letters
        //   (identifying this subset) and "+"

        // Probably won’t use:
        // Word spacing = character spacing for ASCII space 0x20 single-byte code
        // Leading = height between consecutive baselines

        Ok(())
    }

    fn set_alpha(&mut self, alpha: f32) {
        let alpha = alpha.max(0.).min(1.);
        if alpha != self.graphics_state.alpha {
            self.graphics_state.alpha = alpha;

            // Use u16 instead of f32 as a hash key because f32 does not implement Eq,
            // and to do some rounding in case float computation
            // produces very close but different values.
            //
            // Map 0.0 to 0, 1.0 to max
            let hash_key = (alpha * (u16::max_value() as f32)) as u16;

            let next_id = self.doc.alpha_states.len();
            let states = &mut self.doc.extended_graphics_states;
            let pdf_key = self.doc.alpha_states.entry(hash_key).or_insert_with(|| {
                let pdf_key = format!("a{}", next_id);
                states.push((pdf_key.clone().into_bytes(),
                             Object::GraphicsStateDictionaryAlpha(alpha)));
                pdf_key
            });
            op!(self, EXTENDED_GRAPHICS_STATE, &*pdf_key);
        }
    }

    fn add_font(&mut self, font: &Arc<Font>) -> Result<String, FontError> {
        let next_id = self.doc.fonts.len();
        let vacant_entry = match self.doc.fonts.entry(ByAddress(font.clone())) {
            Entry::Occupied(entry) => return Ok(entry.get().clone()),
            Entry::Vacant(entry) => entry,
        };
        let font_bytes = font.bytes();
        let truetype_id = self.doc.pdf.add_stream(
            dictionary! {
                "Length1" => font_bytes.len(),
            },
            font_bytes.into()
        );
        let font_design_units_per_em = font.metrics.font_design_units_per_em.cast::<f32>().unwrap();
        let convert = |x: euclid::Length<i16, FontDesignUnit>| -> euclid::Length<i32, PdfGlyphSpace> {
            (x.cast::<f32>().unwrap() / font_design_units_per_em * glyph_space_units_per_em())
                .cast().unwrap()
        };
        let font_descriptor_id = self.doc.pdf.add_dictionary(dictionary! {
            "Type" => "FontDescriptor",
            "FontName" => &*font.postscript_name,
            "FontBBox" => array![
                convert(font.metrics.min_x).get(),
                convert(font.metrics.min_y).get(),
                convert(font.metrics.max_x).get(),
                convert(font.metrics.max_y).get(),
            ],
            "Ascent" => convert(font.metrics.ascender).get(),
            "Descent" => convert(font.metrics.descender).get(),
            "FontFile2" => truetype_id,

            // These seem somewhat arbitrary, they’re copied from cairo:
            "ItalicAngle" => 0,
            "Flags" => 4,
            "CapHeight" => convert(font.metrics.max_y).get(),
            "StemV" => 80,
            "StemH" => 80,
        });
        // Boilerplate based on a PDF generated by cairo
        let mut to_unicode_cmap = b"\
            /CIDInit /ProcSet findresource begin\n\
            12 dict begin\n\
            begincmap\n\
            /CIDSystemInfo\n\
            << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
            /CMapName /Adobe-Identity-UCS def\n\
            /CMapType 2 def\n\
            1 begincodespacerange\n\
            <0000> <ffff>\n\
            endcodespacerange\n\
        ".to_vec();
        {
            let mut write_bfchar = |chars: &[char], glyph_ids: &[u16]| {
                write!(to_unicode_cmap, "{} beginbfchar\n", chars.len()).unwrap();
                for (ch, glyph_id) in chars.iter().zip(glyph_ids) {
                    write!(to_unicode_cmap, "<{:04x}> <", glyph_id).unwrap();
                    for code_unit in ch.encode_utf16(&mut [0, 0]) {
                        write!(to_unicode_cmap, "{:04x}", code_unit).unwrap()
                    }
                    to_unicode_cmap.extend(b">\n");
                }
                to_unicode_cmap.extend(b"endbfchar\n");
            };
            // Max 100 entries per beginbfchar operator
            let mut chars = ['\0'; 100];
            let mut glyph_ids = [0_u16; 100];
            let mut i = 0;
            font.each_code_point(|ch, GlyphId(glyph_id)| {
                if i >= 100 {
                    write_bfchar(&chars, &glyph_ids);
                    i = 0
                }
                chars[i] = ch;
                glyph_ids[i] = glyph_id;
                i += 1;
            })?;
            if i > 0 {
                write_bfchar(&chars[..i], &glyph_ids[..i])
            }
        }
        to_unicode_cmap.extend(b"\
            endcmap\n\
            CMapName currentdict /CMap defineresource pop\n\
            end\n\
            end\
        ".as_ref());
        let to_unicode_id = self.doc.pdf.add_stream(dictionary! {}, to_unicode_cmap.into());
        // Type 0 Font Dictionaries
        // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G8.1859105

        // FIXME: revert to direct object
        let font_dict_id = self.doc.pdf.add_dictionary(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type0",
            "BaseFont" => &*font.postscript_name,
            "ToUnicode" => to_unicode_id,

            // 2-bytes big-endian char codes, horizontal writing mode:
            "Encoding" => "Identity-H",

            "DescendantFonts" => array![dictionary! {
                "Type" => "Font",
                "Subtype" => "CIDFontType2",
                "BaseFont" => &*font.postscript_name,
                "CIDSystemInfo" => dictionary! {
                    "Registry" => Object::LiteralString(b"Adobe"),
                    "Ordering" => Object::LiteralString(b"Identity"),
                    "Supplement" => 0,
                },
                "FontDescriptor" => font_descriptor_id,
                "W" => array![
                    0,  // start CID
                    &*font.glyph_widths.iter().map(|&width| {
                        let width: euclid::Length<i32, PdfGlyphSpace> = (
                            width.cast::<f32>().unwrap()
                                / font_design_units_per_em
                                * glyph_space_units_per_em()
                        ).cast().unwrap();
                        width.get().into()
                    }).collect::<Vec<Object>>(),
                ],
            }],
        });

        let pdf_key = format!("f{}", next_id);
        self.doc.font_resources.push((pdf_key.clone().into_bytes(), font_dict_id.into()));
        vacant_entry.insert(pdf_key.clone());
        Ok(pdf_key)
    }
}


macro_rules! operators {
    ($( $name: ident = $value: expr, )+) => {
        $(
            const $name: &'static str = $value;
        )+
    }
}

// PDF Content Stream Operators
// https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G14.1032355
operators! {
    // Graphics State Operators
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.3793795
    CURRENT_TRANSFORMATION_MATRIX = "cm",
    EXTENDED_GRAPHICS_STATE = "gs",

    // Path Construction and Painting
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.1849957
    RECTANGLE = "re",
    FILL = "f",

    // Colour Spaces
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.1850197
    NON_STROKING_RGB_COLOR = "rg",

    // Text
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G8.1910927
    BEGIN_TEXT = "BT",
    END_TEXT = "ET",
    TEXT_FONT_AND_SIZE = "Tf",
    TEXT_MATRIX = "Tm",
    SHOW_TEXT = "Tj",
}
