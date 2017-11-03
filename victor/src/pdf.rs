use errors::VictorError;
use display_lists;
use lopdf::{Document, Object, Stream};
use lopdf::content::{Content, Operation};

macro_rules! array {
    ($( $value: expr ),* ,) => {
        array![ $( $value ),* ]
    };
    ($( $value: expr ),*) => {
        vec![ $( Object::from($value) ),* ]
    }
}

pub(crate) fn from_display_lists(dl: &display_lists::Document) -> Result<Document, VictorError> {
    let mut doc = Document::with_version("1.5");
    let page_tree_id = doc.new_object_id();

    let page_ids: Vec<Object> = dl.pages.iter().map(|page| {
        let content = Content { operations: page_content(&page.display_items) };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => page_tree_id,
            "Contents" => content_id,
            "MediaBox" => array![
                0,
                0,
                page.size.width,
                page.size.height,
            ],
        });
        page_id.into()
    }).collect();

    doc.objects.insert(page_tree_id, Object::Dictionary(dictionary! {
        "Type" => "Pages",
        "Count" => page_ids.len() as i64,
        "Kids" => page_ids,
    }));
    let catalog_id = doc.add_object(dictionary!(
        "Type" => "Catalog",
        "Pages" => page_tree_id,
    ));
    let info_id = doc.add_object(dictionary!(
        "Producer" => Object::string_literal("Victor <https://github.com/SimonSapin/victor>"),
    ));

    // PDF file trailer:
    // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1877172
    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);
    Ok(doc)
}

pub fn page_content(display_list: &[display_lists::DisplayItem]) -> Vec<Operation> {
    let operations = Vec::new();
    for display_item in display_list {
        match *display_item {
        }
    }
    operations
}
