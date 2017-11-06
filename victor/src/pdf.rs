use display_lists::*;
use fonts::Font;
use lopdf::{self, Object, Stream, ObjectId, Dictionary, StringFormat};
use lopdf::content::{Content, Operation};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

macro_rules! array {
    ($( $value: expr ),* ,) => {
        array![ $( $value ),* ]
    };
    ($( $value: expr ),*) => {
        vec![ $( Object::from($value) ),* ]
    }
}

const PT_PER_INCH: f32 = 72.;
const PX_PER_INCH: f32 = 96.;
const PT_PER_PX: f32 = PT_PER_INCH / PX_PER_INCH;
const CSS_TO_PDF_SCALE_X: f32 = PT_PER_PX;
const CSS_TO_PDF_SCALE_Y: f32 = -PT_PER_PX;  // Flip the Y axis direction, it defaults to upwards in PDF.

pub(crate) fn from_display_lists(dl: &Document) -> lopdf::Document {
    let mut doc = InProgressDoc {
        pdf: lopdf::Document::with_version("1.5"),
        page_tree_id: (0, 0),
        extended_graphics_states: None,
        font_resources: None,
        alpha_states: HashMap::new(),
        fonts: HashMap::new(),
    };
    doc.page_tree_id = doc.pdf.new_object_id();
    let page_ids: Vec<Object> = dl.pages.iter().map(|p| doc.add_page(p).into()).collect();
    doc.finish(page_ids)
}

struct InProgressDoc {
    pdf: lopdf::Document,
    page_tree_id: ObjectId,
    extended_graphics_states: Option<Dictionary>,
    font_resources: Option<Dictionary>,
    alpha_states: HashMap<u16, String>,
    fonts: HashMap<usize, String>,
}

impl InProgressDoc {
    fn finish(mut self, page_ids: Vec<Object>) -> lopdf::Document {
        let mut page_tree = dictionary! {
            "Type" => "Pages",
            "Count" => page_ids.len() as i64,
            "Kids" => page_ids,
        };

        let mut resources = None;
        if let Some(fonts) = self.font_resources {
            resources.get_or_insert_with(Dictionary::new).set("Font", fonts)
        }
        if let Some(states) = self.extended_graphics_states {
            resources.get_or_insert_with(Dictionary::new).set("ExtGState", states)
        }
        if let Some(resources) = resources {
            page_tree.set("Resources", resources)
        }

        self.pdf.objects.insert(self.page_tree_id, Object::Dictionary(page_tree));
        let catalog_id = self.pdf.add_object(dictionary!(
            "Type" => "Catalog",
            "Pages" => self.page_tree_id,
        ));
        let info_id = self.pdf.add_object(dictionary!(
            "Producer" => Object::string_literal("Victor <https://github.com/SimonSapin/victor>"),
        ));

        // PDF file trailer:
        // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1941947
        self.pdf.trailer.set("Root", catalog_id);
        self.pdf.trailer.set("Info", info_id);
        self.pdf
    }

    fn add_page(&mut self, page: &Page) -> ObjectId {
        let content = {
            let mut in_progress = InProgressPage {
                doc: self,
                operations: Vec::new(),
                // Initial state:
                graphics_state: GraphicsState {
                    non_stroking_color_rgb: (0., 0., 0.),  // Black
                    alpha: 1.,  // Fully opaque
                },
            };
            in_progress.add_content(&page.display_items);
            Content { operations: in_progress.operations }
        };
        let content_id = self.pdf.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        self.pdf.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => self.page_tree_id,
            "Contents" => content_id,
            "MediaBox" => array![
                0,
                0,
                page.size.width * CSS_TO_PDF_SCALE_X,
                page.size.height * CSS_TO_PDF_SCALE_Y,
            ],
        })
    }
}

struct InProgressPage<'a> {
    doc: &'a mut InProgressDoc,
    operations: Vec<Operation>,
    graphics_state: GraphicsState,
}

struct GraphicsState {
    non_stroking_color_rgb: (f32, f32, f32),
    alpha: f32,
}

macro_rules! op {
    ( $self_: expr, $operator: expr ) => {
        op!($self_, $operator,)
    };
    ( $self_: expr, $operator: expr, $( $operands: tt )*) => {
        $self_.operations.push(Operation::new($operator, array![ $($operands)* ]))
    }
}

impl<'a> InProgressPage<'a> {
    fn add_content(&mut self, display_list: &[DisplayItem]) {
        op!(self, CURRENT_TRANSFORMATION_MATRIX, CSS_TO_PDF_SCALE_X, 0, 0, CSS_TO_PDF_SCALE_Y, 0, 0);
        for display_item in display_list {
            match *display_item {
                DisplayItem::SolidRectangle(ref rect, ref rgba) => {
                    self.set_non_stroking_color(rgba);
                    op!(self, RECTANGLE, rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
                    op!(self, FILL);
                }

                DisplayItem::Text { ref font, ref font_size, ref color, ref start, ref glyph_ids } => {
                    self.set_non_stroking_color(color);
                    let font_key = self.add_font(font);
                    // flip the Y axis in to compensate the same flip at the page level.
                    let x_scale = font_size.0;
                    let y_scale = -font_size.0;
                    let mut glyph_codes = Vec::with_capacity(glyph_ids.len() * 2);
                    for &id in glyph_ids {
                        // Big-endian
                        glyph_codes.push((id >> 8) as u8);
                        glyph_codes.push(id as u8);
                    }
                    op!(self, BEGIN_TEXT);
                    op!(self, TEXT_FONT_AND_SIZE, font_key, 1);
                    op!(self, TEXT_MATRIX, x_scale, 0, 0, y_scale, start.x, start.y);
                    op!(self, SHOW_TEXT, Object::String(glyph_codes, StringFormat::Hexadecimal));
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
                }
            }
        }
    }

    fn set_non_stroking_color(&mut self, &RGBA(r, g, b, a): &RGBA) {
        if self.graphics_state.non_stroking_color_rgb != (r, g, b) {
            self.graphics_state.non_stroking_color_rgb = (r, g, b);
            op!(self, NON_STROKING_RGB_COLOR, r, g, b);
        }
        self.set_alpha(a)
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
                states.get_or_insert_with(Dictionary::new).set(
                    pdf_key.clone(),
                    dictionary! {
                        "CA" => alpha,
                        "ca" => alpha,
                    },
                );
                pdf_key
            });
            op!(self, EXTENDED_GRAPHICS_STATE, pdf_key.clone());
        }
    }

    fn add_font(&mut self, font: &Arc<Font>) -> String {
        let ptr: *const Font = &**font;
        let hash_key = ptr as usize;
        let InProgressDoc { ref mut pdf, ref mut font_resources, ref mut fonts, .. } = *self.doc;
        let next_id = fonts.len();
        let pdf_key = fonts.entry(hash_key).or_insert_with(|| {
            let truetype_id = pdf.add_object(Stream::new(
                dictionary! {
                    "Length1" => font.bytes.len() as i64,
                },
                font.bytes.to_vec()
            ));
            let font_descriptor_id = pdf.add_object(dictionary! {
                "Type" => "FontDescriptor",
                "FontName" => &*font.postscript_name,
                "FontBBox" => array![
                    font.min_x,
                    font.min_y,
                    font.max_x,
                    font.max_y,
                ],
                "Ascent" => font.ascent,
                "Descent" => font.descent,
                "FontFile2" => truetype_id,

                // These seem somewhat arbitrary, they’re copied from cairo:
                "ItalicAngle" => 0,
                "Flags" => 4,
                "CapHeight" => font.max_y,
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
            let mut pairs: Vec<_> = font.cmap.iter().map(|(&k, &v)| (k, v)).collect();
            pairs.sort();
            // Max 100 entries per beginbfchar operator
            for chunk in pairs.chunks(100) {
                write!(to_unicode_cmap, "{} beginbfchar\n", chunk.len()).unwrap();
                for &(code_point, glyph) in chunk {
                    write!(to_unicode_cmap, "<{:04x}> <{:04x}>\n", glyph, code_point).unwrap();
                }
                to_unicode_cmap.extend(b"endbfchar\n");
            }
            to_unicode_cmap.extend(b"\
                endcmap\n\
                CMapName currentdict /CMap defineresource pop\n\
                end\n\
                end\n\
            ".as_ref());
            let to_unicode_id = pdf.add_object(Stream::new(dictionary! {}, to_unicode_cmap));
            // Type 0 Font Dictionaries
            // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G8.1859105
            let font_dict = dictionary! {
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
                        "Registry" => Object::string_literal("Adobe"),
                        "Ordering" => Object::string_literal("Identity"),
                        "Supplement" => 0,
                    },
                    "FontDescriptor" => font_descriptor_id,
                    "W" => array![
                        0,  // start CID
                        font.glyph_widths.iter().map(|&w| w.into()).collect::<Vec<Object>>(),
                    ],
                }],
            };

            let pdf_key = format!("f{}", next_id);
            font_resources.get_or_insert_with(Dictionary::new).set(pdf_key.clone(), font_dict);
            pdf_key
        });
        pdf_key.clone()
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
