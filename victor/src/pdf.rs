use errors::VictorError;
use display_lists;
use lopdf::{Document, Object, Dictionary};
use std::iter::FromIterator;

pub(crate) fn from_display_lists(_dl: &display_lists::Document) -> Result<Document, VictorError> {
    let mut doc = Document::with_version("1.5");
    let catalog_id = doc.add_object(
        Dictionary::from_iter(vec![
            ("Type", "Catalog".into()),
        ])
    );
    doc.trailer.set("Root", Object::Reference(catalog_id));
    Ok(doc)
}
