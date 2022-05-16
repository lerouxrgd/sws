use std::cell::Ref;
use std::rc::Rc;

use crate::{Node, NodeId, NodeRef, Tree};

/// Iterator that moves out of a tree in insert order.
#[derive(Debug)]
pub struct IntoIter<T>(slotmap::basic::IntoIter<NodeId, Node<T>>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().and_then(|(_node_id, node)| {
            Rc::try_unwrap(node.value).map(|val| val.into_inner()).ok()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> IntoIterator for Tree<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.sm.into_inner().into_iter())
    }
}

/// Iterator over nodes in insert order.
pub struct Nodes<'a, T: 'a> {
    r: Ref<'a, slotmap::SlotMap<NodeId, Node<T>>>,
}

impl<'a, 'b: 'a, T: 'a> IntoIterator for &'b Nodes<'a, T> {
    type IntoIter = slotmap::basic::Iter<'a, NodeId, Node<T>>;
    type Item = (NodeId, &'a Node<T>);

    fn into_iter(self) -> slotmap::basic::Iter<'a, NodeId, Node<T>> {
        self.r.iter()
    }
}

impl<T> Tree<T> {
    pub fn try_into_iter(self: Rc<Self>) -> Result<IntoIter<T>, Rc<Self>> {
        Rc::try_unwrap(self).map(|tree| tree.into_iter())
    }

    pub fn nodes(&self) -> Nodes<T> {
        Nodes {
            r: self.sm.borrow(),
        }
    }
}

macro_rules! axis_iterators {
    ($(#[$m:meta] $i:ident($f:path);)*) => {
        $(
            #[$m]
            #[derive(Debug, Clone)]
            pub struct $i<T>(Option<NodeRef<T>>);

            impl<T> Iterator for $i<T> {
                type Item = NodeRef<T>;

                fn next(&mut self) -> Option<Self::Item> {
                    let node = self.0.take();
                    self.0 = node.as_ref().and_then($f);
                    node
                }
            }
        )*
    };
}

axis_iterators! {
    /// Iterator over ancestors.
    Ancestors(NodeRef::parent);

    /// Iterator over previous siblings.
    PrevSiblings(NodeRef::prev_sibling);

    /// Iterator over next siblings.
    NextSiblings(NodeRef::next_sibling);

    /// Iterator over first children.
    FirstChildren(NodeRef::first_child);

    /// Iterator over last children.
    LastChildren(NodeRef::last_child);
}

/// Iterator over children.
#[derive(Debug)]
pub struct Children<T> {
    front: Option<NodeRef<T>>,
    back: Option<NodeRef<T>>,
}

impl<T> Clone for Children<T> {
    fn clone(&self) -> Self {
        Self {
            front: self.front.clone(),
            back: self.back.clone(),
        }
    }
}

impl<T> Iterator for Children<T> {
    type Item = NodeRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front == self.back {
            let node = self.front.take();
            self.back = None;
            node
        } else {
            let node = self.front.take();
            self.front = node.as_ref().and_then(NodeRef::next_sibling);
            node
        }
    }
}

impl<T> DoubleEndedIterator for Children<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.back == self.front {
            let node = self.back.take();
            self.front = None;
            node
        } else {
            let node = self.back.take();
            self.back = node.as_ref().and_then(NodeRef::prev_sibling);
            node
        }
    }
}

/// Open or close edge of a node.
#[derive(Debug)]
pub enum Edge<T> {
    /// Open.
    Open(NodeRef<T>),
    /// Close.
    Close(NodeRef<T>),
}

impl<T> Clone for Edge<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Close(node) => Self::Close(node.clone()),
            Self::Open(node) => Self::Open(node.clone()),
        }
    }
}

impl<T> PartialEq for Edge<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Edge::Open(a), Edge::Open(b)) | (Edge::Close(a), Edge::Close(b)) => a == b,
            _ => false,
        }
    }
}

/// Iterator which traverses a subtree.
#[derive(Debug)]
pub struct Traverse<T> {
    root: NodeRef<T>,
    edge: Option<Edge<T>>,
}

impl<T> Clone for Traverse<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            edge: self.edge.clone(),
        }
    }
}

impl<T> Iterator for Traverse<T> {
    type Item = Edge<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.edge {
            None => {
                self.edge = Some(Edge::Open(self.root.clone()));
            }
            Some(Edge::Open(node)) => {
                if let Some(first_child) = node.first_child() {
                    self.edge = Some(Edge::Open(first_child));
                } else {
                    self.edge = Some(Edge::Close(node.clone()));
                }
            }
            Some(Edge::Close(node)) => {
                if node == &self.root {
                    self.edge = None;
                } else if let Some(next_sibling) = node.next_sibling() {
                    self.edge = Some(Edge::Open(next_sibling));
                } else {
                    self.edge = node.parent().map(Edge::Close);
                }
            }
        }

        self.edge.clone()
    }
}

/// Iterator over a node and its descendants.
#[derive(Debug)]
pub struct Descendants<T>(Traverse<T>);

impl<T> Clone for Descendants<T> {
    fn clone(&self) -> Self {
        Descendants(self.0.clone())
    }
}

impl<T> Iterator for Descendants<T> {
    type Item = NodeRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        for edge in &mut self.0 {
            if let Edge::Open(node) = edge {
                return Some(node);
            }
        }
        None
    }
}

impl<T> NodeRef<T> {
    /// Returns an iterator over ancestors.
    pub fn ancestors(&self) -> Ancestors<T> {
        Ancestors(self.parent())
    }

    /// Returns an iterator over previous siblings.
    pub fn prev_siblings(&self) -> PrevSiblings<T> {
        PrevSiblings(self.prev_sibling())
    }

    /// Returns an iterator over next siblings.
    pub fn next_siblings(&self) -> NextSiblings<T> {
        NextSiblings(self.next_sibling())
    }

    /// Returns an iterator over first children.
    pub fn first_children(&self) -> FirstChildren<T> {
        FirstChildren(self.first_child())
    }

    /// Returns an iterator over last children.
    pub fn last_children(&self) -> LastChildren<T> {
        LastChildren(self.last_child())
    }

    /// Returns an iterator over children.
    pub fn children(&self) -> Children<T> {
        Children {
            front: self.first_child(),
            back: self.last_child(),
        }
    }

    /// Returns an iterator which traverses the subtree starting at this node.
    pub fn traverse(&self) -> Traverse<T> {
        Traverse {
            root: self.clone(),
            edge: None,
        }
    }

    /// Returns an iterator over this node and its descendants.
    pub fn descendants(&self) -> Descendants<T> {
        Descendants(self.traverse())
    }
}
