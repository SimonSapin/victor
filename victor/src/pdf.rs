use errors::VictorError;
use display_lists;
use lopdf::{Document, Object, Dictionary};

macro_rules! dict {
    ($( $key: expr => $value: expr ),+ ,) => {
        dict!( $($key => $value),+ )
    };
    ($( $key: expr => $value: expr ),*) => {{
        let mut dict = Dictionary::new();
        $(
            dict.set($key, $value);
        )*
        dict
    }}
}

pub(crate) fn from_display_lists(_dl: &display_lists::Document) -> Result<Document, VictorError> {
    let mut doc = Document::with_version("1.5");
    let catalog_id = doc.add_object(dict!(
        "Type" => "Catalog",
    ));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    Ok(doc)
}
