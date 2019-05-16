use super::{Document, NodeId};

pub(crate) struct PseudoElements<P> {
    pub before: Option<P>,
    pub after: Option<P>,
}

enum TreeDirection<P> {
    NextSibling,
    FirstChild,
    PseudoBeforeThenFirstChild(P),
}

pub(crate) enum TreeItem<P> {
    Node(NodeId),
    PseudoElement(P),
}

struct SubtreeCursor<'a, P> {
    document: &'a Document,
    node_id: NodeId,
    next_direction: TreeDirection<P>,
    after_pseudo_elements_for_ancestors: smallvec::SmallVec<[Option<P>; 8]>,
}

impl<'a, P> SubtreeCursor<'a, P> {
    fn for_descendendants_of(
        node_id: NodeId,
        pseudos: PseudoElements<P>,
        document: &'a Document,
    ) -> Self {
        let PseudoElements { before, after } = pseudos;
        Self {
            node_id,
            document,
            after_pseudo_elements_for_ancestors: std::iter::once(after).collect(),
            next_direction: match before {
                Some(p) => TreeDirection::PseudoBeforeThenFirstChild(p),
                None => TreeDirection::FirstChild,
            },
        }
    }

    /// Move to the next node an return its ID.
    ///
    /// Return `None` if there are no more nodes at this nesting level.
    fn next(&mut self) -> Option<TreeItem<P>> {
        let node = &self.document[self.node_id];
        let direction = std::mem::replace(&mut self.next_direction, TreeDirection::NextSibling);
        let next_item = match direction {
            TreeDirection::NextSibling => node.next_sibling.map(TreeItem::Node),
            TreeDirection::FirstChild => node.first_child.map(TreeItem::Node),
            TreeDirection::PseudoBeforeThenFirstChild(pseudo) => {
                self.next_direction = TreeDirection::FirstChild;
                Some(TreeItem::PseudoElement(pseudo))
            }
        };
        if let Some(TreeItem::Node(id)) = next_item {
            self.node_id = id
        }
        next_item.or_else(|| {
            self.after_pseudo_elements_for_ancestors
                .last_mut()
                .expect("called next after move_to_parent returned Err")
                .take()
                .map(TreeItem::PseudoElement)
        })
    }

    /// Move one nesting level deeper, to the child nodes on the current node
    fn traverse_children_of_this_node(&mut self, pseudos: PseudoElements<P>) {
        assert!(matches!(self.next_direction, TreeDirection::NextSibling));
        let PseudoElements { before, after } = pseudos;
        self.after_pseudo_elements_for_ancestors.push(after);
        self.next_direction = match before {
            Some(p) => TreeDirection::PseudoBeforeThenFirstChild(p),
            None => TreeDirection::FirstChild,
        }
    }

    /// Move back “up” one nesting level.
    ///
    /// To be called after `self.next()` returns `None`.
    ///
    /// Return `Err(())` if we were already at the initial nesting level.
    fn move_to_parent(&mut self) -> Result<(), ()> {
        if self.after_pseudo_elements_for_ancestors.len() <= 1 {
            return Err(());
        }
        debug_assert!(self
            .after_pseudo_elements_for_ancestors
            .pop()
            .unwrap()
            .is_none());
        match self.next_direction {
            TreeDirection::NextSibling => {
                let node = &self.document[self.node_id];
                self.node_id = node.parent.expect("moving past the root");
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
            TreeDirection::PseudoBeforeThenFirstChild(_) => {
                self.next_direction = TreeDirection::NextSibling;
                debug_assert!(false, "missed some nodes in DOM tree traversal")
            }
        }
        Ok(())
    }
}

pub(crate) struct SubtreeCursorWithDisplayContents<'a, P> {
    cursor: SubtreeCursor<'a, P>,
    ancestor_stack: smallbitvec::SmallBitVec,
}

impl<'a, P> SubtreeCursorWithDisplayContents<'a, P> {
    pub fn for_descendendants_of(
        node_id: NodeId,
        pseudos: PseudoElements<P>,
        document: &'a Document,
    ) -> Self {
        Self {
            cursor: SubtreeCursor::for_descendendants_of(node_id, pseudos, document),
            ancestor_stack: smallbitvec::SmallBitVec::new(),
        }
    }

    /// Move to the next node an return its ID.
    ///
    /// Return `None` if there are no more nodes at this (apparent) nesting level.
    pub fn next(&mut self) -> Option<TreeItem<P>> {
        loop {
            let next = self.cursor.next();
            if next.is_none() {
                // We’ve moved past the last sibling in this actual nesting level.

                // Look at the status of the parent.
                // If there isn’t any, we’re done with the subtree
                // of the node passed to `for_descendendants_of`.
                let pretend_children_are_siblings = self.ancestor_stack.last()?;

                if pretend_children_are_siblings {
                    // This parent had `display: contents`: move the actual nesting level up
                    // without changing the apparent nesting level.
                    self.cursor.move_to_parent().unwrap();
                    continue;
                }
            }
            return next;
        }
    }

    /// Move one nesting level deeper, to the child nodes on the current node
    pub fn traverse_children_of_this_node(&mut self, pseudos: PseudoElements<P>) {
        self.cursor.traverse_children_of_this_node(pseudos);
        self.ancestor_stack.push(false);
    }

    /// Behave as if the children of this node were following siblings.
    ///
    /// Do not change the apparent nesting level.
    /// This is used for implementing `display: contents`.
    pub fn pretend_children_are_siblings(&mut self, pseudos: PseudoElements<P>) {
        self.cursor.traverse_children_of_this_node(pseudos);
        self.ancestor_stack.push(true);
    }

    /// After traversing the current apparent nesting level, resume one level up
    /// with the following siblings of the parent node (if any).
    ///
    /// Return `Err(())` if we were already at the initial nesting level.
    pub fn move_to_parent(&mut self) -> Result<(), ()> {
        loop {
            self.cursor.move_to_parent().map_err(|()| {
                // We’re done with the subtree of the node passed to `for_descendendants_of`.
                debug_assert!(self.ancestor_stack.is_empty());
            })?;
            let pretend_children_are_siblings = self.ancestor_stack.pop().unwrap();
            if !pretend_children_are_siblings {
                // Found a nesting level that was not `display: contents`.
                return Ok(());
            }
        }
    }
}
