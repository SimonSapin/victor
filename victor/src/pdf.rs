use errors::VictorError;
use display_lists;
use lopdf::{Document, Object};

pub(crate) fn from_display_lists(dl: &display_lists::Document) -> Result<Document, VictorError> {
    let mut doc = Document::with_version("1.5");
    let page_tree_id = doc.new_object_id();

    let page_ids: Vec<Object> = dl.pages.iter().map(|page| {
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => page_tree_id,
            "MediaBox" => vec![
                0.into(),
                0.into(),
                page.width_in_ps_points.into(),
                page.height_in_ps_points.into()
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
    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);
    Ok(doc)
}
