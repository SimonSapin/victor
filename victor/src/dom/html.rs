use super::*;
use html5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{self, parse_document, ExpandedName};
use std::borrow::Cow;
use std::collections::HashSet;

impl Document {
    pub fn parse_html(utf8_bytes: &[u8]) -> Self {
        let sink = Sink {
            document: Document::new(),
            quirks_mode: QuirksMode::NoQuirks,
        };
        parse_document(sink, Default::default())
            .from_utf8()
            .one(utf8_bytes)
    }
}

struct Sink {
    document: Document,
    quirks_mode: QuirksMode,
}

impl Sink {
    fn new_node(&mut self, data: NodeData) -> NodeId {
        self.document.push_node(Node::new(data))
    }

    fn append_common<P, A>(&mut self, child: NodeOrText<NodeId>, previous: P, append: A)
    where
        P: FnOnce(&mut Document) -> Option<NodeId>,
        A: FnOnce(&mut Document, NodeId),
    {
        let new_node = match child {
            NodeOrText::AppendText(text) => {
                // Append to an existing Text node if we have one.
                if let Some(id) = previous(&mut self.document) {
                    if let Node {
                        data: NodeData::Text { contents },
                        ..
                    } = &mut self.document[id]
                    {
                        contents.push_str(&text);
                        return
                    }
                }
                self.new_node(NodeData::Text {
                    contents: text.into(),
                })
            }
            NodeOrText::AppendNode(node) => node,
        };

        append(&mut self.document, new_node)
    }
}

impl TreeSink for Sink {
    type Handle = NodeId;
    type Output = Document;

    fn finish(self) -> Document {
        self.document
    }

    fn parse_error(&mut self, _: Cow<'static, str>) {}

    fn get_document(&mut self) -> NodeId {
        Document::document_node_id()
    }

    fn set_quirks_mode(&mut self, mode: QuirksMode) {
        self.quirks_mode = mode;
    }

    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        x == y
    }

    fn elem_name<'a>(&'a self, &target: &'a NodeId) -> ExpandedName<'a> {
        self.document[target]
            .as_element()
            .expect("not an element")
            .name
            .expanded()
    }

    fn get_template_contents(&mut self, &target: &NodeId) -> NodeId {
        target
    }

    fn is_mathml_annotation_xml_integration_point(&self, &target: &NodeId) -> bool {
        self.document[target]
            .as_element()
            .expect("not an element")
            .mathml_annotation_xml_integration_point
    }

    fn create_element(
        &mut self,
        name: QualName,
        attrs: Vec<html5ever::Attribute>,
        ElementFlags {
            mathml_annotation_xml_integration_point,
            ..
        }: ElementFlags,
    ) -> NodeId {
        let is_style = name.expanded() == expanded_name!(html "style");
        let element = self.new_node(NodeData::Element(ElementData {
            name,
            attrs: attrs.into_iter().map(Attribute::from).collect(),
            mathml_annotation_xml_integration_point,
        }));
        if is_style {
            self.document.style_elements.push(element)
        }
        element
    }

    fn create_comment(&mut self, text: StrTendril) -> NodeId {
        self.new_node(NodeData::Comment {
            _contents: text.into(),
        })
    }

    fn create_pi(&mut self, target: StrTendril, data: StrTendril) -> NodeId {
        self.new_node(NodeData::ProcessingInstruction {
            _target: target.into(),
            _contents: data.into(),
        })
    }

    fn append(&mut self, &parent: &NodeId, child: NodeOrText<NodeId>) {
        self.append_common(
            child,
            |document| document[parent].last_child,
            |document, new_node| document.append(parent, new_node),
        )
    }

    fn append_before_sibling(&mut self, &sibling: &NodeId, child: NodeOrText<NodeId>) {
        self.append_common(
            child,
            |document| document[sibling].previous_sibling,
            |document, new_node| document.insert_before(sibling, new_node),
        )
    }

    fn append_based_on_parent_node(
        &mut self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        if self.document[*element].parent.is_some() {
            self.append_before_sibling(element, child)
        } else {
            self.append(prev_element, child)
        }
    }

    fn append_doctype_to_document(
        &mut self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let node = self.new_node(NodeData::Doctype {
            _name: name.into(),
            _public_id: public_id.into(),
            _system_id: system_id.into(),
        });
        self.document.append(Document::document_node_id(), node)
    }

    fn add_attrs_if_missing(&mut self, &target: &NodeId, attrs: Vec<html5ever::Attribute>) {
        let element = if let NodeData::Element(element) = &mut self.document[target].data {
            element
        } else {
            panic!("not an element")
        };
        let existing_names = element
            .attrs
            .iter()
            .map(|e| e.name.clone())
            .collect::<HashSet<_>>();
        element.attrs.extend(
            attrs
                .into_iter()
                .map(Attribute::from)
                .filter(|attr| !existing_names.contains(&attr.name)),
        );
    }

    fn remove_from_parent(&mut self, &target: &NodeId) {
        self.document.detach(target)
    }

    fn reparent_children(&mut self, &node: &NodeId, &new_parent: &NodeId) {
        let mut next_child = self.document[node].first_child;
        while let Some(child) = next_child {
            debug_assert_eq!(self.document[child].parent, Some(node));
            self.document.append(new_parent, child);
            next_child = self.document[child].next_sibling
        }
    }
}

impl From<html5ever::Attribute> for Attribute {
    fn from(attr: html5ever::Attribute) -> Self {
        Self {
            name: attr.name,
            value: attr.value.into(),
        }
    }
}
