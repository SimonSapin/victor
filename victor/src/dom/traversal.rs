use super::{Document, ElementData, NodeData, NodeId};
use crate::layout::ReplacedContent;
use crate::style::values::{Display, DisplayGeneratingBox, DisplayInside, DisplayOutside};
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
    fn handle_element(
        &mut self,
        style: &Arc<ComputedValues>,
        display: DisplayGeneratingBox,
        contents: Contents,
    ) -> TreeDirection;

    fn move_to_parent(&mut self);
}

pub(crate) enum TreeDirection {
    SkipThisSubtree,
    TraverseChildren,
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
            NodeData::Element(data) => {
                traverse_element(child, data, parent_element_style, context, handler)
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

fn traverse_element(
    element_id: NodeId,
    element_data: &ElementData,
    parent_element_style: &ComputedValues,
    context: &Context,
    handler: &mut impl Handler,
) {
    let style = style_for_element(
        context.author_styles,
        context.document,
        element_id,
        Some(parent_element_style),
    );
    let display_self = match style.box_.display {
        Display::None => return,
        Display::Contents => None,
        Display::GeneratingBox(display) => Some(display),
    };
    if let Some(replaced) = ReplacedContent::for_element(element_id, element_data, context) {
        if let Some(display) = display_self {
            let dir = handler.handle_element(&style, display, Contents::Replaced(replaced));
            assert!(
                matches!(dir, TreeDirection::SkipThisSubtree),
                "children of a replaced element should be ignored"
            );
        } else {
            // `display: content` on a replaced element computes to `display: none`
            // <https://drafts.csswg.org/css-display-3/#valdef-display-contents>
        }
    } else {
        // Non-replaced element
        if let Some(display) = display_self {
            let dir = handler.handle_element(&style, display, Contents::OfElement(element_id));
            match dir {
                TreeDirection::SkipThisSubtree => {}
                TreeDirection::TraverseChildren => {
                    traverse_children_of(element_id, &style, context, handler);
                    handler.move_to_parent()
                }
            }
        } else {
            // `display: content`
            traverse_children_of(element_id, &style, context, handler);
        }
    }
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
        let display_self = match pseudo_element_style.box_.display {
            Display::None => return,
            Display::Contents => None,
            Display::GeneratingBox(display) => Some(display),
        };
        let items = generate_pseudo_element_content(pseudo_element_style, element, context);
        let contents = Contents::OfPseudoElement(items.clone());
        if let Some(display) = display_self {
            let direction = handler.handle_element(pseudo_element_style, display, contents);
            match direction {
                TreeDirection::SkipThisSubtree => {}
                TreeDirection::TraverseChildren => {
                    traverse_pseudo_element_contents(pseudo_element_style, items, handler);
                    handler.move_to_parent()
                }
            }
        } else {
            // `display: contents`
            traverse_pseudo_element_contents(pseudo_element_style, items, handler);
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
                let display_inline = DisplayGeneratingBox::OutsideInside {
                    outside: DisplayOutside::Inline,
                    inside: DisplayInside::Flow,
                };
                // `display` is not inherited, so we get the initial value
                debug_assert!(item_style.box_.display == Display::GeneratingBox(display_inline));
                let direction = handler.handle_element(
                    item_style,
                    display_inline,
                    Contents::Replaced(contents.clone()),
                );
                assert!(
                    matches!(direction, TreeDirection::SkipThisSubtree),
                    "children of a replaced element should be ignored"
                );
            }
        }
    }
}

impl ReplacedContent {
    fn for_element(_element: NodeId, _data: &ElementData, _context: &Context) -> Option<Arc<Self>> {
        // FIXME: implement <img> etc.
        None
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
