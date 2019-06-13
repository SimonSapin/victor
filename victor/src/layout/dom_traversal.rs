use super::*;
use crate::dom::{Document, NodeData, NodeId};
use crate::style::StyleSet;

pub(super) struct Context<'a> {
    pub document: &'a Document,
    pub author_styles: &'a StyleSet,
}

pub(super) enum Contents {
    /// Refers to a DOM subtree, plus `::before` and `::after` pseudo-elements.
    OfElement(NodeId),

    /// Example: an `<img src=…>` element.
    /// <https://drafts.csswg.org/css2/conform.html#replaced-element>
    Replaced(ReplacedContent),

    /// Content of a `::before` or `::after` pseudo-element this is being generated.
    /// <https://drafts.csswg.org/css2/generate.html#content>
    OfPseudoElement(Vec<PseudoElementContentItem>),
}

pub(super) enum NonReplacedContents {
    OfElement(NodeId),
    OfPseudoElement(Vec<PseudoElementContentItem>),
}

pub(super) enum PseudoElementContentItem {
    Text(String),
    Replaced(ReplacedContent),
}

pub(super) trait TraversalHandler {
    fn handle_text(&mut self, text: &str, parent_style: &Arc<ComputedValues>);

    /// Or pseudo-element
    fn handle_element(
        &mut self,
        style: &Arc<ComputedValues>,
        display: DisplayGeneratingBox,
        contents: Contents,
    );
}

#[allow(unused)]
fn traverse_children_of(
    parent_element: NodeId,
    parent_element_style: &Arc<ComputedValues>,
    context: &Context,
    handler: &mut impl TraversalHandler,
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
            NodeData::Element(_) => traverse_element(child, parent_element_style, context, handler),
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
    parent_element_style: &ComputedValues,
    context: &Context,
    handler: &mut impl TraversalHandler,
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
    if let Some(replaced) = ReplacedContent::for_element(element_id, context) {
        if let Some(display) = display_self {
            handler.handle_element(&style, display, Contents::Replaced(replaced))
        } else {
            // `display: content` on a replaced element computes to `display: none`
            // <https://drafts.csswg.org/css-display-3/#valdef-display-contents>
        }
    } else {
        // Non-replaced element
        if let Some(display) = display_self {
            handler.handle_element(&style, display, Contents::OfElement(element_id))
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
    handler: &mut impl TraversalHandler,
) {
    let result = pseudo_element_style(which, element, element_style, context);
    if let Some(pseudo_element_style) = &result {
        let display_self = match pseudo_element_style.box_.display {
            Display::None => return,
            Display::Contents => None,
            Display::GeneratingBox(display) => Some(display),
        };
        let items = generate_pseudo_element_content(pseudo_element_style, element, context);
        if let Some(display) = display_self {
            let contents = Contents::OfPseudoElement(items);
            handler.handle_element(pseudo_element_style, display, contents)
        } else {
            // `display: contents`
            traverse_pseudo_element_contents(pseudo_element_style, items, handler);
        }
    }
}

fn traverse_pseudo_element_contents(
    pseudo_element_style: &Arc<ComputedValues>,
    items: Vec<PseudoElementContentItem>,
    handler: &mut impl TraversalHandler,
) {
    let mut anonymous_style = None;
    for item in items {
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
                handler.handle_element(item_style, display_inline, Contents::Replaced(contents))
            }
        }
    }
}

impl std::convert::TryFrom<Contents> for NonReplacedContents {
    type Error = ReplacedContent;

    fn try_from(contents: Contents) -> Result<Self, Self::Error> {
        match contents {
            Contents::OfElement(id) => Ok(NonReplacedContents::OfElement(id)),
            Contents::OfPseudoElement(items) => Ok(NonReplacedContents::OfPseudoElement(items)),
            Contents::Replaced(replaced) => Err(replaced),
        }
    }
}

impl std::convert::From<NonReplacedContents> for Contents {
    fn from(contents: NonReplacedContents) -> Self {
        match contents {
            NonReplacedContents::OfElement(id) => Contents::OfElement(id),
            NonReplacedContents::OfPseudoElement(items) => Contents::OfPseudoElement(items),
        }
    }
}

impl NonReplacedContents {
    pub fn traverse(
        self,
        inherited_style: &Arc<ComputedValues>,
        context: &Context,
        handler: &mut impl TraversalHandler,
    ) {
        match self {
            NonReplacedContents::OfElement(id) => {
                traverse_children_of(id, inherited_style, context, handler)
            }
            NonReplacedContents::OfPseudoElement(items) => {
                traverse_pseudo_element_contents(inherited_style, items, handler)
            }
        }
    }
}

impl ReplacedContent {
    pub(super) fn for_element(_element: NodeId, _context: &Context) -> Option<Self> {
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
    _pseudo_element_style: &ComputedValues,
    _element: NodeId,
    _context: &Context,
) -> Vec<PseudoElementContentItem> {
    let _ = PseudoElementContentItem::Text;
    let _ = PseudoElementContentItem::Replaced;
    unimplemented!()
}