use super::{Document, ElementData, NodeData, NodeId};
use crate::layout::ReplacedContent;
use crate::style::{style_for_element, ComputedValues, StyleSet};
use std::sync::Arc;

pub(crate) struct Context<'a> {
    pub document: &'a Document,
    pub author_styles: &'a StyleSet,
}

pub(crate) enum Contents {
    /// Refers to a DOM subtree, plus `::before` and `::after` pseudo-elements.
    OfElement(NodeId),

    /// Example: an `<img src=…>` element.
    /// <https://drafts.csswg.org/css2/conform.html#replaced-element>
    Replaced(Arc<ReplacedContent>),

    /// Content of a `::before` or `::after` pseudo-element this is being generated.
    /// <https://drafts.csswg.org/css2/generate.html#content>
    OfPseudoElement(Arc<Vec<PseudoElementContentItem>>),
}

pub(crate) enum PseudoElementContentItem {
    Text(String),
    Replaced(Arc<ReplacedContent>),
}

pub(crate) trait Handler {
    fn handle_text(&mut self, text: &str, parent_style: &Arc<ComputedValues>);

    /// Or pseudo-element
    fn handle_element(&mut self, style: &Arc<ComputedValues>, contents: Contents) -> TreeDirection;

    fn move_to_parent(&mut self);
}

pub(crate) enum TreeDirection {
    SkipThisSubtree,

    TraverseChildren,

    /// `display: contents`
    PretendChildrenAreSiblings,
}

#[allow(unused)]
pub(crate) fn traverse_children_of(
    parent_element: NodeId,
    parent_element_style: &Arc<ComputedValues>,
    context: &Context,
    handler: &mut impl Handler,
) {
    traverse_pseudo_element(
        WhichPseudoElement::Before,
        parent_element,
        parent_element_style,
        context,
        handler,
    );

    let mut next = context.document[parent_element].first_child;
    while let Some(child) = next {
        match &context.document[child].data {
            NodeData::Document
            | NodeData::Doctype { .. }
            | NodeData::Comment { .. }
            | NodeData::ProcessingInstruction { .. } => {}
            NodeData::Text { contents } => {
                handler.handle_text(contents, parent_element_style);
            }
            NodeData::Element(element_data) => {
                let style = style_for_element(
                    context.author_styles,
                    context.document,
                    child,
                    Some(parent_element_style),
                );
                match ReplacedContent::for_element(child, element_data, context) {
                    Some(replaced) => {
                        let dir = handler.handle_element(&style, Contents::Replaced(replaced));
                        assert!(
                            matches!(dir, TreeDirection::SkipThisSubtree),
                            "children of a replaced element should be ignored"
                        );
                    }
                    None => match handler.handle_element(&style, Contents::OfElement(child)) {
                        TreeDirection::SkipThisSubtree => {}
                        TreeDirection::TraverseChildren => {
                            traverse_children_of(child, &style, context, handler);
                            handler.move_to_parent()
                        }
                        TreeDirection::PretendChildrenAreSiblings => {
                            traverse_children_of(child, &style, context, handler);
                        }
                    },
                }
            }
        }
        next = context.document[child].next_sibling
    }

    traverse_pseudo_element(
        WhichPseudoElement::After,
        parent_element,
        &parent_element_style,
        context,
        handler,
    );
}

fn traverse_pseudo_element(
    which: WhichPseudoElement,
    element: NodeId,
    element_style: &ComputedValues,
    context: &Context,
    handler: &mut impl Handler,
) {
    let result = pseudo_element_style(which, element, element_style, context);
    if let Some(pseudo_element_style) = &result {
        let items = generate_pseudo_element_content(pseudo_element_style, element, context);
        let contents = Contents::OfPseudoElement(items.clone());
        let direction = handler.handle_element(pseudo_element_style, contents);
        let pretend_children_are_siblings = match direction {
            TreeDirection::SkipThisSubtree => return,
            TreeDirection::TraverseChildren => false,
            TreeDirection::PretendChildrenAreSiblings => true,
        };
        traverse_pseudo_element_contents(pseudo_element_style, items, handler);
        if !pretend_children_are_siblings {
            handler.move_to_parent()
        }
    }
}

pub(crate) fn traverse_pseudo_element_contents(
    pseudo_element_style: &Arc<ComputedValues>,
    items: Arc<Vec<PseudoElementContentItem>>,
    handler: &mut impl Handler,
) {
    let mut anonymous_style = None;
    for item in &*items {
        match item {
            PseudoElementContentItem::Text(text) => {
                handler.handle_text(&text, pseudo_element_style)
            }
            PseudoElementContentItem::Replaced(contents) => {
                let item_style = anonymous_style.get_or_insert_with(|| {
                    ComputedValues::anonymous_inheriting_from(Some(pseudo_element_style))
                });
                let direction =
                    handler.handle_element(item_style, Contents::Replaced(contents.clone()));
                // There are no children
                match direction {
                    TreeDirection::SkipThisSubtree => {}
                    TreeDirection::PretendChildrenAreSiblings => {}
                    TreeDirection::TraverseChildren => handler.move_to_parent(),
                }
            }
        }
    }
}

enum WhichPseudoElement {
    Before,
    After,
}

fn pseudo_element_style(
    _which: WhichPseudoElement,
    _element: NodeId,
    _element_style: &ComputedValues,
    _context: &Context,
) -> Option<Arc<ComputedValues>> {
    // FIXME: run the cascade, then return None for `content: normal` or `content: none`
    // https://drafts.csswg.org/css2/generate.html#content
    None
}

fn generate_pseudo_element_content(
    _style: &ComputedValues,
    _element: NodeId,
    _context: &Context,
) -> Arc<Vec<PseudoElementContentItem>> {
    let _ = PseudoElementContentItem::Text;
    let _ = PseudoElementContentItem::Replaced;
    unimplemented!()
}
