use super::{Document, NodeId};

#[derive(PartialEq)]
enum TreeDirection {
    NextSibling,
    FirstChild,
}

pub(crate) struct SubtreeCursor<'a> {
    document: &'a Document,
    node_id: NodeId,
    next_direction: TreeDirection,
    ancestor_stack: smallbitvec::SmallBitVec,
}

impl<'a> SubtreeCursor<'a> {
    pub fn for_descendendants_of(node_id: NodeId, document: &'a Document) -> Self {
        Self {
            node_id,
            next_direction: TreeDirection::FirstChild,
            document,
            ancestor_stack: smallbitvec::SmallBitVec::new(),
        }
    }

    /// Move to the next node an return its ID.
    ///
    /// Return `None` if there are no more nodes at this (apparent) nesting level.
    pub fn next(&mut self) -> Option<NodeId> {
        loop {
            let node = &self.document[self.node_id];
            let next = match self.next_direction {
                TreeDirection::NextSibling => node.next_sibling,
                TreeDirection::FirstChild => node.first_child,
            };
            if let Some(id) = next {
                self.next_direction = TreeDirection::NextSibling;
                self.node_id = id
            } else {
                let pretend_children_are_siblings = self.ancestor_stack.last()?;
                if pretend_children_are_siblings {
                    self.move_to_actual_parent();
                    continue;
                }
            }

            return next;
        }
    }

    /// Move one nesting level deeper, to the child nodes on the current node
    pub fn traverse_children_of_this_node(&mut self) {
        assert!(self.next_direction == TreeDirection::NextSibling);
        self.next_direction = TreeDirection::FirstChild;
        self.ancestor_stack.push(false);
    }

    /// Behave as if the children of this node were following siblings.
    /// Do not change the apparent nesting level.
    pub fn pretend_children_are_siblings(&mut self) {
        assert!(self.next_direction == TreeDirection::NextSibling);
        self.next_direction = TreeDirection::FirstChild;
        self.ancestor_stack.push(true);
    }

    fn move_to_actual_parent(&mut self) {
        self.ancestor_stack.pop();
        match self.next_direction {
            TreeDirection::NextSibling => {
                let node = &self.document[self.node_id];
                self.node_id = node.parent.expect("child node without a parent");
                debug_assert!(
                    node.next_sibling.is_none(),
                    "missed some nodes in DOM tree traversal"
                );
            }
            TreeDirection::FirstChild => {
                self.next_direction = TreeDirection::NextSibling;
                debug_assert!(
                    self.document[self.node_id].first_child.is_none(),
                    "missed some nodes in DOM tree traversal"
                );
            }
        }
    }

    /// After traversing the current apparent nesting level, resume one level up
    /// with the following siblings of the parent node (if any).
    ///
    /// Return `Err(())` if we were already at the initial (apparent) nesting level.
    pub fn move_to_parent(&mut self) -> Result<(), ()> {
        loop {
            let pretend_children_are_siblings = self.ancestor_stack.last().ok_or(())?;
            self.move_to_actual_parent();
            if !pretend_children_are_siblings {
                return Ok(());
            }
        }
    }
}
