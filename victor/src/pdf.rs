use display_lists::*;
use lopdf::{self, Object, Stream};
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
    let mut pdf_doc = lopdf::Document::with_version("1.5");
    let page_tree_id = pdf_doc.new_object_id();

    let page_ids: Vec<Object> = dl.pages.iter().map(|page| {
        let content = Content { operations: page_content(&page.display_items) };
        let content_id = pdf_doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = pdf_doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => page_tree_id,
            "Contents" => content_id,
            "MediaBox" => array![
                0,
                0,
                page.size.width * CSS_TO_PDF_SCALE_X,
                page.size.height * CSS_TO_PDF_SCALE_Y,
            ],
        });
        page_id.into()
    }).collect();

    pdf_doc.objects.insert(page_tree_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Count" => page_ids.len() as i64,
        "Kids" => page_ids,
    }));
    let catalog_id = pdf_doc.add_object(dictionary!(
        "Type" => "Catalog",
        "Pages" => page_tree_id,
    ));
    let info_id = pdf_doc.add_object(dictionary!(
        "Producer" => Object::string_literal("Victor <https://github.com/SimonSapin/victor>"),
    ));

    // PDF file trailer:
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1941947
    pdf_doc.trailer.set("Root", catalog_id);
    pdf_doc.trailer.set("Info", info_id);
    pdf_doc
}

pub fn page_content(display_list: &[DisplayItem]) -> Vec<Operation> {
    let mut operations = Vec::new();

    macro_rules! op {
        ( $operator: expr ) => {
            op!($operator,)
        };
        ( $operator: expr, $( $operands: tt )*) => {
            operations.push(Operation {
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
    operations
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
