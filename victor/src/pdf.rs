use errors::VictorError;
use display_lists;
use lopdf::{Document, Object};

pub(crate) fn from_display_lists(_dl: &display_lists::Document) -> Result<Document, VictorError> {
    let mut doc = Document::with_version("1.5");
    let catalog_id = doc.add_object(dictionary!(
        "Type" => "Catalog",
    ));
    let info_id = doc.add_object(dictionary!(
        "Producer" => Object::string_literal("Victor <https://github.com/SimonSapin/victor>"),
    ));
    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);
    Ok(doc)
}
