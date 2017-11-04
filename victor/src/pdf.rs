use display_lists::*;
use lopdf::{self, Object, Stream, ObjectId};
use lopdf::content::{Content, Operation};

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
    };
    let page_ids: Vec<Object> = dl.pages.iter().map(|p| doc.add_page(p).into()).collect();
    doc.finish(page_ids)
}

struct InProgressPdf {
    doc: lopdf::Document,
    page_tree_id: ObjectId,
}

impl InProgressPdf {
    fn finish(mut self, page_ids: Vec<Object>) -> lopdf::Document {
        self.doc.objects.insert(self.page_tree_id, Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => page_ids.len() as i64,
            "Kids" => page_ids,
        }));
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
}

impl<'a> InProgressPage<'a> {
    fn add_content(&mut self, display_list: &[DisplayItem]) {
        macro_rules! op {
            ( $operator: expr ) => {
                op!($operator,)
            };
            ( $operator: expr, $( $operands: tt )*) => {
                self.operations.push(Operation {
                    operator: $operator.into(),
                    operands: array![ $($operands)* ],
                })
            }
        }

        op!(CURRENT_TRANSFORMATION_MATRIX, CSS_TO_PDF_SCALE_X, 0, 0, CSS_TO_PDF_SCALE_Y, 0, 0);
        for display_item in display_list {
            match *display_item {
                // FIXME: Whenever we add text, flip the Y axis in the text transformation matrix
                // to compensate the same flip at the page level.
                DisplayItem::SolidRectangle(ref rect, RGB(red, green, blue)) => {
                    op!(SET_NON_STROKING_RGB_COLOR, red, green, blue);
                    op!(RECTANGLE, rect.origin.x, rect.origin.y, rect.size.width, rect.size.height);
                    op!(FILL);
                }
            }
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

    // Path Construction and Painting
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.1849957
    RECTANGLE = "re",
    FILL = "f",

    // Colour Spaces
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G7.1850197
    SET_NON_STROKING_RGB_COLOR = "rg",
}
