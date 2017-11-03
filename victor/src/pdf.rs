use errors::VictorError;
use display_lists;
use lopdf::Document;

pub(crate) fn from_display_lists(_dl: &display_lists::Document) -> Result<Document, VictorError> {
    let mut doc = Document::with_version("1.5");
    let catalog_id = doc.add_object(dictionary!(
        "Type" => "Catalog",
    ));
    doc.trailer.set("Root", catalog_id);
    Ok(doc)
}
