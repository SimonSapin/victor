use html5ever::{parse_document, ExpandedName};
use html5ever::tendril::TendrilSink;
use html5ever::interface::tree_builder::{TreeSink, QuirksMode, NodeOrText, ElementFlags};
use std::borrow::Cow;
use std::collections::HashSet;
use super::*;

impl<'arena> Document<'arena> {
    pub fn parse_html(utf8_bytes: &[u8], arena: ArenaRef<'arena>) -> Self {
        let sink = Sink {
            arena: arena,
            document: Document {
                document_node: arena.allocate(Node::new(NodeData::Document)),
                style_elements: Vec::new(),
            },
            quirks_mode: QuirksMode::NoQuirks,
        };
        parse_document(sink, Default::default()).from_utf8().one(utf8_bytes)
    }
}

struct Sink<'arena> {
    arena: ArenaRef<'arena>,
    document: Document<'arena>,
    quirks_mode: QuirksMode,
}

impl<'arena> Sink<'arena> {
    fn new_node(&self, data: NodeData<'arena>) -> NodeRef<'arena> {
        self.arena.allocate(Node::new(data))
    }

    fn append_common<P, A>(&self, child: NodeOrText<NodeRef<'arena>>, previous: P, append: A)
        where P: FnOnce() -> Option<NodeRef<'arena>>,
              A: FnOnce(NodeRef<'arena>),
    {
        let new_node = match child {
            NodeOrText::AppendText(text) => {
                // Append to an existing Text node if we have one.
                if let Some(&Node { data: NodeData::Text { ref contents }, .. }) = previous() {
                    contents.borrow_mut().push_tendril(&text);
                    return
                }
                self.new_node(NodeData::Text { contents: RefCell::new(text) })
            }
            NodeOrText::AppendNode(node) => node
        };

        append(new_node)
    }
}

impl<'arena> TreeSink for Sink<'arena> {
    type Handle = NodeRef<'arena>;
    type Output = Document<'arena>;

    fn finish(self) -> Document<'arena> {
        self.document
    }

    fn parse_error(&mut self, _: Cow<'static, str>) {}

    fn get_document(&mut self) -> NodeRef<'arena> {
        self.document.document_node
    }

    fn set_quirks_mode(&mut self, mode: QuirksMode) {
        self.quirks_mode = mode;
    }

    fn same_node(&self, x: &NodeRef<'arena>, y: &NodeRef<'arena>) -> bool {
        ptr::eq::<Node>(*x, *y)
    }

    fn elem_name<'a>(&self, target: &'a NodeRef<'arena>) -> ExpandedName<'a> {
        match target.data {
            NodeData::Element { ref name, .. } => name.expanded(),
            _ => panic!("not an element!"),
        }
    }

    fn get_template_contents(&mut self, target: &NodeRef<'arena>) -> NodeRef<'arena> {
        if let NodeData::Element { template_contents: Some(ref contents), .. } = target.data {
            contents
        } else {
            panic!("not a template element!")
        }
    }

    fn is_mathml_annotation_xml_integration_point(&self, target: &NodeRef<'arena>) -> bool {
        if let NodeData::Element { mathml_annotation_xml_integration_point, .. } = target.data {
            mathml_annotation_xml_integration_point
        } else {
            panic!("not an element!")
        }
    }

    fn create_element(&mut self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags)
                      -> NodeRef<'arena> {
        let is_style = name.expanded() == expanded_name!(html "style");
        let element = self.new_node(NodeData::Element {
            name: name,
            attrs: RefCell::new(attrs),
            template_contents: if flags.template {
                Some(self.new_node(NodeData::Document))
            } else {
                None
            },
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,

        });
        if is_style {
            self.document.style_elements.push(element)
        }
        element
    }

    fn create_comment(&mut self, text: StrTendril) -> NodeRef<'arena> {
        self.new_node(NodeData::Comment { contents: text })
    }

    fn create_pi(&mut self, target: StrTendril, data: StrTendril) -> NodeRef<'arena> {
        self.new_node(NodeData::ProcessingInstruction { target: target, contents: data })
    }

    fn append(&mut self, parent: &NodeRef<'arena>, child: NodeOrText<NodeRef<'arena>>) {
        self.append_common(
            child,
            || parent.last_child.get(),
            |new_node| parent.append(new_node)
        )
    }

    fn append_before_sibling(&mut self, sibling: &NodeRef<'arena>,
                             child: NodeOrText<NodeRef<'arena>>) {
        self.append_common(
            child,
            || sibling.previous_sibling.get(),
            |new_node| sibling.insert_before(new_node)
        )
    }

    fn append_based_on_parent_node(&mut self, element: &NodeRef<'arena>,
                                   prev_element: &NodeRef<'arena>,
                                   child: NodeOrText<NodeRef<'arena>>) {
        if element.parent.get().is_some() {
            self.append_before_sibling(element, child)
        } else {
            self.append(prev_element, child)
        }
    }

    fn append_doctype_to_document(&mut self,
                                  name: StrTendril,
                                  public_id: StrTendril,
                                  system_id: StrTendril) {
        self.document.append(self.new_node(NodeData::Doctype {
            name: name,
            public_id: public_id,
            system_id: system_id
        }))
    }

    fn add_attrs_if_missing(&mut self, target: &NodeRef<'arena>, attrs: Vec<Attribute>) {
        let mut existing = if let NodeData::Element { ref attrs, .. } = target.data {
            attrs.borrow_mut()
        } else {
            panic!("not an element")
        };

        let existing_names = existing.iter().map(|e| e.name.clone()).collect::<HashSet<_>>();
        existing.extend(attrs.into_iter().filter(|attr| {
            !existing_names.contains(&attr.name)
        }));
    }

    fn remove_from_parent(&mut self, target: &NodeRef<'arena>) {
        target.detach()
    }

    fn reparent_children(&mut self, node: &NodeRef<'arena>, new_parent: &NodeRef<'arena>) {
        let mut next_child = node.first_child.get();
        while let Some(child) = next_child {
            debug_assert!(ptr::eq::<Node>(child.parent.get().unwrap(), *node));
            next_child = child.next_sibling.get();
            new_parent.append(child)
        }
    }
}
