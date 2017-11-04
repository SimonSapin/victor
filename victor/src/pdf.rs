use display_lists::*;
use lopdf::{self, Object, Stream, ObjectId, Dictionary};
use lopdf::content::{Content, Operation};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

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
    let mut doc = lopdf::Document::with_version("1.5");
    let mut doc = InProgressPdf {
        page_tree_id: doc.new_object_id(),
        doc,
        extended_graphics_states: None,
        alpha_states: HashMap::new(),
    };
    let page_ids: Vec<Object> = dl.pages.iter().map(|p| doc.add_page(p).into()).collect();
    doc.finish(page_ids)
}

struct InProgressPdf {
    doc: lopdf::Document,
    page_tree_id: ObjectId,
    extended_graphics_states: Option<Dictionary>,
    alpha_states: HashMap<u16, usize>,
}

impl InProgressPdf {
    fn finish(mut self, page_ids: Vec<Object>) -> lopdf::Document {
        let mut page_tree = dictionary! {
            "Type" => "Pages",
            "Count" => page_ids.len() as i64,
            "Kids" => page_ids,
        };
        if let Some(states) = self.extended_graphics_states {
            page_tree.set("Resources", dictionary! {
                "ExtGState" => states,
            })
        }
        self.doc.objects.insert(self.page_tree_id, Object::Dictionary(page_tree));
        let catalog_id = self.doc.add_object(dictionary!(
            "Type" => "Catalog",
            "Pages" => self.page_tree_id,
        ));
        let info_id = self.doc.add_object(dictionary!(
            "Producer" => Object::string_literal("Victor <https://github.com/SimonSapin/victor>"),
        ));

        // PDF file trailer:
        // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1941947
        self.doc.trailer.set("Root", catalog_id);
        self.doc.trailer.set("Info", info_id);
        self.doc
    }

    fn add_page(&mut self, page: &Page) -> ObjectId {
        let content = {
            let mut in_progress = InProgressPage {
                doc: self,
                operations: Vec::new(),
                // Initial state:
                graphics_state: GraphicsState {
                    non_stroking_color: RGB(0., 0., 0.),  // Black
                    alpha: 1.,
                },
            };
            in_progress.add_content(&page.display_items);
            Content { operations: in_progress.operations }
        };
        let content_id = self.doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        self.doc.add_object(dictionary! {
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
    doc: &'a mut InProgressPdf,
    operations: Vec<Operation>,
    graphics_state: GraphicsState,
}

struct GraphicsState {
    non_stroking_color: RGB,
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
                // FIXME: Whenever we add text, flip the Y axis in the text transformation matrix
                // to compensate the same flip at the page level.
                DisplayItem::SolidRectangle(ref rect, ref rgb) => {
                    self.set_non_stroking_color(rgb);
                    op!(self, RECTANGLE, rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
                    op!(self, FILL);
                }
            }
        }
    }

    fn set_non_stroking_color(&mut self, rgba: &RGBA) {
        let rgb = &rgba.rgb;
        if *rgb != self.graphics_state.non_stroking_color {
            self.graphics_state.non_stroking_color = *rgb;
            op!(self, SET_NON_STROKING_RGB_COLOR, rgb.0, rgb.1, rgb.2);
        }
        self.set_alpha(rgba.alpha)
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
            let pdf_state_key;
            match self.doc.alpha_states.entry(hash_key) {
                Entry::Occupied(entry) => {
                    pdf_state_key = format!("a{}", entry.get());
                }
                Entry::Vacant(_) => {
                    pdf_state_key = format!("a{}", next_id);
                    self.doc.extended_graphics_states.get_or_insert_with(Dictionary::new).set(
                        &*pdf_state_key,
                        dictionary! {
                            "CA" => alpha,
                            "ca" => alpha,
                        }
                    );
                }
            }
            op!(self, SET_EXTENDED_GRAPHICS_STATE, pdf_state_key);
        }
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
    SET_EXTENDED_GRAPHICS_STATE = "gs",

    // Path Construction and Painting
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.1849957
    RECTANGLE = "re",
    FILL = "f",

    // Colour Spaces
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.1850197
    SET_NON_STROKING_RGB_COLOR = "rg",
}
