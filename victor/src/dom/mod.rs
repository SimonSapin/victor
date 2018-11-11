//! This is *a* Document Object Model, but is not necessarily compatible with *the* DOM.

mod html;

use crate::style::StyleSetBuilder;
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, ExpandedName, LocalName, QualName};
use std::borrow::Cow;
use std::fmt;

pub struct Document {
    nodes: Vec<Node>,
    style_elements: Vec<NodeId>,
}

pub struct Node {
    pub(crate) parent: Option<NodeId>,
    pub(crate) next_sibling: Option<NodeId>,
    pub(crate) previous_sibling: Option<NodeId>,
    pub(crate) first_child: Option<NodeId>,
    pub(crate) last_child: Option<NodeId>,
    pub(crate) data: NodeData,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodeId(std::num::NonZeroUsize);

impl Document {
    fn new() -> Self {
        // Dummy node at index 0 so that other indices fit in NonZero
        let dummy = Node::new(NodeData::Document);

        let document_node = Node::new(NodeData::Document);
        Document {
            nodes: vec![dummy, document_node],
            style_elements: Vec::new(),
        }
    }

    fn document_node_id() -> NodeId {
        NodeId(std::num::NonZeroUsize::new(1).unwrap())
    }

    pub fn parse_stylesheets(&self, style_set: &mut StyleSetBuilder) {
        for &id in &self.style_elements {
            let element = &self[id];
            // https://html.spec.whatwg.org/multipage/semantics.html#update-a-style-block
            if let Some(type_attr) = element.as_element().unwrap().get_attr(&local_name!("type")) {
                if !type_attr.eq_ignore_ascii_case("text/css") {
                    continue
                }
            }
            style_set.add_stylesheet(&self.child_text_content(id))
        }
    }

    fn push_node(&mut self, node: Node) -> NodeId {
        let next_index = self.nodes.len();
        self.nodes.push(node);
        NodeId(std::num::NonZeroUsize::new(next_index).unwrap())
    }

    fn detach(&mut self, node: NodeId) {
        let (parent, previous_sibling, next_sibling) = {
            let node = &mut self[node];
            (
                node.parent.take(),
                node.previous_sibling.take(),
                node.next_sibling.take(),
            )
        };

        if let Some(next_sibling) = next_sibling {
            self[next_sibling].previous_sibling = previous_sibling
        } else if let Some(parent) = parent {
            self[parent].last_child = previous_sibling;
        }

        if let Some(previous_sibling) = previous_sibling {
            self[previous_sibling].next_sibling = next_sibling;
        } else if let Some(parent) = parent {
            self[parent].first_child = next_sibling;
        }
    }

    fn append(&mut self, parent: NodeId, new_child: NodeId) {
        self.detach(new_child);
        self[new_child].parent = Some(parent);
        if let Some(last_child) = self[parent].last_child.take() {
            self[new_child].previous_sibling = Some(last_child);
            debug_assert!(self[last_child].next_sibling.is_none());
            self[last_child].next_sibling = Some(new_child);
        } else {
            debug_assert!(self[parent].first_child.is_none());
            self[parent].first_child = Some(new_child);
        }
        self[parent].last_child = Some(new_child);
    }

    fn insert_before(&mut self, sibling: NodeId, new_sibling: NodeId) {
        self.detach(new_sibling);
        self[new_sibling].parent = self[sibling].parent;
        self[new_sibling].next_sibling = Some(sibling);
        if let Some(previous_sibling) = self[sibling].previous_sibling.take() {
            self[new_sibling].previous_sibling = Some(previous_sibling);
            debug_assert_eq!(self[previous_sibling].next_sibling, Some(sibling));
            self[previous_sibling].next_sibling = Some(new_sibling);
        } else if let Some(parent) = self[sibling].parent {
            debug_assert_eq!(self[parent].first_child, Some(sibling));
            self[parent].first_child = Some(new_sibling);
        }
        self[sibling].previous_sibling = Some(new_sibling);
    }

    /// <https://dom.spec.whatwg.org/#concept-child-text-content>
    fn child_text_content(&self, node: NodeId) -> Cow<StrTendril> {
        let mut link = self[node].first_child;
        let mut text = None;
        while let Some(child) = link {
            if let NodeData::Text { contents } = &self[child].data {
                match &mut text {
                    None => text = Some(Cow::Borrowed(contents)),
                    Some(text) => text.to_mut().push_tendril(&contents),
                }
            }
            link = self[child].next_sibling;
        }
        text.unwrap_or_else(|| Cow::Owned(StrTendril::new()))
    }

    pub(crate) fn root_element(&self) -> NodeId {
        let first_child;
        {
            let document_node = &self[Document::document_node_id()];
            assert!(matches!(document_node.data, NodeData::Document));
            assert!(document_node.parent.is_none());
            assert!(document_node.next_sibling.is_none());
            assert!(document_node.previous_sibling.is_none());
            first_child = document_node.first_child
        }
        let mut root = None;
        for child in self.node_and_next_siblings(first_child.unwrap()) {
            match &self[child].data {
                NodeData::Doctype { .. }
                | NodeData::Comment { .. }
                | NodeData::ProcessingInstruction { .. } => {}
                NodeData::Document | NodeData::Text { .. } => {
                    panic!("Unexpected node type under document node")
                }
                NodeData::Element(_) => {
                    assert!(root.is_none(), "Found two root elements");
                    root = Some(child)
                }
            }
        }
        root.unwrap()
    }

    pub(crate) fn node_and_next_siblings<'a>(
        &'a self,
        node: NodeId,
    ) -> impl Iterator<Item = NodeId> + 'a {
        successors(Some(node), move |&node| self[node].next_sibling)
    }
}

impl std::ops::Index<NodeId> for Document {
    type Output = Node;

    #[inline]
    fn index(&self, id: NodeId) -> &Node {
        &self.nodes[id.0.get()]
    }
}

impl std::ops::IndexMut<NodeId> for Document {
    #[inline]
    fn index_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.0.get()]
    }
}

pub(crate) enum NodeData {
    Document,
    Doctype {
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    },
    Text {
        contents: StrTendril,
    },
    Comment {
        _contents: StrTendril,
    },
    Element(ElementData),
    ProcessingInstruction {
        _target: StrTendril,
        _contents: StrTendril,
    },
}

pub(crate) struct ElementData {
    pub(crate) name: QualName,
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) mathml_annotation_xml_integration_point: bool,
}

impl ElementData {
    pub(crate) fn get_attr(&self, name: &LocalName) -> Option<&StrTendril> {
        let name = ExpandedName {
            ns: &ns!(),
            local: name,
        };
        self.attrs.iter().find_map(|attr| {
            if attr.name.expanded() == name {
                Some(&attr.value)
            } else {
                None
            }
        })
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn size_of() {
    use std::mem::size_of;
    assert_eq!(size_of::<Node>(), 112);
    assert_eq!(size_of::<NodeData>(), 72);
    assert_eq!(size_of::<ElementData>(), 64);
}

impl Node {
    pub(crate) fn in_html_document(&self) -> bool {
        // FIXME: track something when we add XML parsing
        true
    }

    pub(crate) fn as_element(&self) -> Option<&ElementData> {
        match self.data {
            NodeData::Element(ref data) => Some(data),
            _ => None,
        }
    }

    pub(crate) fn as_text(&self) -> Option<&StrTendril> {
        match self.data {
            NodeData::Text { ref contents } => Some(contents),
            _ => None,
        }
    }

    fn new(data: NodeData) -> Self {
        Node {
            parent: None,
            previous_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            data: data,
        }
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ptr: *const Node = self;
        f.debug_tuple("Node").field(&ptr).finish()
    }
}

fn successors<T, F>(first: Option<T>, mut succ: F) -> impl Iterator<Item = T>
where
    F: FnMut(&T) -> Option<T>,
{
    unfold(first, move |next| {
        next.take().map(|item| {
            *next = succ(&item);
            item
        })
    })
}

fn unfold<T, St, F>(initial_state: St, f: F) -> Unfold<St, F>
where
    F: FnMut(&mut St) -> Option<T>,
{
    Unfold {
        state: initial_state,
        f,
    }
}

struct Unfold<St, F> {
    state: St,
    f: F,
}

impl<T, St, F> Iterator for Unfold<St, F>
where
    F: FnMut(&mut St) -> Option<T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        (self.f)(&mut self.state)
    }
}
