//! [SlotMap](https://crates.io/crates/slotmap)-backed ID-tree.
//!
//! Port of [ego-tree](https://crates.io/crates/ego-tree), but using [`Rc`](std::rc::Rc)
//! instead of references with lifetimes, and without using `unsafe`.

#![allow(clippy::option_map_unit_fn)]

pub mod iter;

use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};

use slotmap::{new_key_type, Key, SlotMap};

new_key_type! {
    pub struct NodeId;
}

/// Slotmap-backed ID-tree.
///
/// Always contains at least a root node.
#[derive(Debug)]
pub struct Tree<T> {
    root: NodeId,
    sm: RefCell<SlotMap<NodeId, Node<T>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Node<T> {
    parent: NodeId,
    prev_sibling: NodeId,
    next_sibling: NodeId,
    children: (NodeId, NodeId),
    value: Rc<RefCell<T>>,
}

impl<T> PartialEq for Tree<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.nodes()
            .into_iter()
            .zip(&other.nodes())
            .all(|((_, v1), (_, v2))| *v1.value() == *v2.value())
    }
}

impl<T> Node<T> {
    pub fn new(value: T) -> Self {
        Node {
            parent: NodeId::null(),
            prev_sibling: NodeId::null(),
            next_sibling: NodeId::null(),
            children: (NodeId::null(), NodeId::null()),
            value: Rc::new(RefCell::new(value)),
        }
    }

    pub fn value(&self) -> Ref<'_, T> {
        self.value.borrow()
    }

    pub fn value_mut(&self) -> RefMut<'_, T> {
        self.value.borrow_mut()
    }
}

impl<T> Tree<T> {
    /// Creates a tree with a root node.
    pub fn new(root: T) -> Rc<Self> {
        let mut sm = SlotMap::with_key();
        let root = sm.insert(Node::new(root));
        Rc::new(Tree {
            root,
            sm: RefCell::new(sm),
        })
    }

    /// Creates a tree with a root node and the specified capacity.
    pub fn with_capacity(root: T, capacity: usize) -> Rc<Self> {
        let mut sm = SlotMap::with_capacity_and_key(capacity);
        let root = sm.insert(Node::new(root));
        Rc::new(Tree {
            root,
            sm: RefCell::new(sm),
        })
    }

    /// Returns a reference to the specified node.
    pub fn get(self: &Rc<Self>, id: NodeId) -> Option<NodeRef<T>> {
        self.sm.borrow().get(id).map(|_node| NodeRef {
            id,
            tree: Rc::downgrade(self),
        })
    }

    /// Returns a reference to the root node.
    pub fn root(self: &Rc<Self>) -> NodeRef<T> {
        self.get(self.root).unwrap()
    }

    /// Creates an orphan node.
    pub fn orphan(self: &Rc<Self>, value: T) -> NodeId {
        self.sm.borrow_mut().insert(Node::new(value))
    }
}

/// Node reference.
#[derive(Debug)]
pub struct NodeRef<T> {
    id: NodeId,
    tree: Weak<Tree<T>>,
}

impl<T> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            tree: self.tree.clone(),
        }
    }
}

impl<T> PartialEq for NodeRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.tree.strong_count() > 0
            && other.tree.strong_count() > 0
            && self.tree.upgrade().map(|rc| &*rc as *const _)
                == other.tree.upgrade().map(|rc| &*rc as *const _)
    }
}

impl<T> NodeRef<T> {
    /// Returns the ID of this node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns the result of map_fn applied to the value of this node.
    pub fn map_value<F, R>(&self, map_fn: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        self.tree.upgrade().and_then(|tree| {
            tree.sm
                .borrow()
                .get(self.id)
                .map(|node| map_fn(&node.value.borrow()))
        })
    }

    /// Update the value of this node.
    pub fn update_value<F>(&self, update_fn: F) -> bool
    where
        F: FnOnce(&mut T),
    {
        self.tree
            .upgrade()
            .and_then(|tree| {
                tree.sm.borrow().get(self.id).map(|node| {
                    update_fn(&mut node.value.borrow_mut());
                    true
                })
            })
            .unwrap_or(false)
    }

    /// Returns the parent of this node.
    pub fn parent(&self) -> Option<Self> {
        self.tree.upgrade().and_then(|tree| {
            tree.sm
                .borrow()
                .get(self.id)
                .and_then(|node| tree.get(node.parent))
        })
    }

    /// Returns the previous sibling of this node.
    pub fn prev_sibling(&self) -> Option<Self> {
        self.tree.upgrade().and_then(|tree| {
            tree.sm
                .borrow()
                .get(self.id)
                .and_then(|node| tree.get(node.prev_sibling))
        })
    }

    /// Returns the next sibling of this node.
    pub fn next_sibling(&self) -> Option<Self> {
        self.tree.upgrade().and_then(|tree| {
            tree.sm
                .borrow()
                .get(self.id)
                .and_then(|node| tree.get(node.next_sibling))
        })
    }

    /// Returns the first child of this node.
    pub fn first_child(&self) -> Option<Self> {
        self.tree.upgrade().and_then(|tree| {
            tree.sm
                .borrow()
                .get(self.id)
                .and_then(|node| tree.get(node.children.0))
        })
    }

    /// Returns the last child of this node.
    pub fn last_child(&self) -> Option<Self> {
        self.tree.upgrade().and_then(|tree| {
            tree.sm
                .borrow()
                .get(self.id)
                .and_then(|node| tree.get(node.children.1))
        })
    }

    /// Returns true if this node has siblings.
    pub fn has_siblings(&self) -> bool {
        self.tree
            .upgrade()
            .map(|tree| {
                tree.sm
                    .borrow()
                    .get(self.id)
                    .map(|node| !node.prev_sibling.is_null() || !node.next_sibling.is_null())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Returns true if this node has children.
    pub fn has_children(&self) -> bool {
        self.tree
            .upgrade()
            .map(|tree| {
                tree.sm
                    .borrow()
                    .get(self.id)
                    .map(|node| !node.children.0.is_null() && !node.children.1.is_null())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Detaches this node from its parent.
    pub fn detach(&mut self) {
        self.tree.upgrade().map(|tree| {
            let (parent_id, prev_sibling_id, next_sibling_id) = tree
                .sm
                .borrow()
                .get(self.id)
                .map(|node| (node.parent, node.prev_sibling, node.next_sibling))
                .unwrap_or((NodeId::null(), NodeId::null(), NodeId::null()));

            if !parent_id.is_null() {
                tree.sm.borrow_mut().get_mut(self.id).map(|node| {
                    node.parent = NodeId::null();
                    node.prev_sibling = NodeId::null();
                    node.next_sibling = NodeId::null();
                });
            } else {
                return;
            }

            if !prev_sibling_id.is_null() {
                tree.sm
                    .borrow_mut()
                    .get_mut(prev_sibling_id)
                    .map(|node| node.next_sibling = next_sibling_id);
            }

            if !next_sibling_id.is_null() {
                tree.sm
                    .borrow_mut()
                    .get_mut(next_sibling_id)
                    .map(|node| node.prev_sibling = prev_sibling_id);
            }

            tree.sm.borrow_mut().get_mut(parent_id).map(|parent| {
                let (first_child_id, last_child_id) = parent.children;
                if first_child_id == last_child_id {
                    parent.children = (NodeId::null(), NodeId::null());
                } else if first_child_id == self.id {
                    parent.children = (next_sibling_id, last_child_id);
                } else if last_child_id == self.id {
                    parent.children = (first_child_id, prev_sibling_id);
                }
            });
        });
    }

    /// Appends a new child to this node.
    pub fn append(&mut self, value: T) -> Option<NodeRef<T>> {
        self.tree.upgrade().and_then(|tree| {
            let new_child_id = tree.orphan(value);
            self.append_id(new_child_id)
        })
    }

    /// Appends a child to this node.
    pub fn append_id(&mut self, new_child_id: NodeId) -> Option<NodeRef<T>> {
        self.tree.upgrade().and_then(|tree| {
            let last_child_id = self
                .last_child()
                .map(|child| child.id)
                .unwrap_or_else(NodeId::null);

            tree.sm
                .borrow_mut()
                .get_mut(new_child_id)
                .map(|new_child| {
                    new_child.parent = self.id;
                    new_child.prev_sibling = last_child_id;
                });

            tree.sm
                .borrow_mut()
                .get_mut(last_child_id)
                .map(|last_child| {
                    last_child.next_sibling = new_child_id;
                });

            tree.sm.borrow_mut().get_mut(self.id).map(|this_node| {
                if !this_node.children.0.is_null() {
                    this_node.children.1 = new_child_id;
                } else {
                    this_node.children.0 = new_child_id;
                    this_node.children.1 = new_child_id;
                }
            });

            tree.get(new_child_id)
        })
    }

    /// Prepends a new child to this node.
    pub fn prepend(&mut self, value: T) -> Option<NodeRef<T>> {
        self.tree.upgrade().and_then(|tree| {
            let new_child_id = tree.orphan(value);

            let first_child_id = self
                .first_child()
                .map(|child| child.id)
                .unwrap_or_else(NodeId::null);

            tree.sm
                .borrow_mut()
                .get_mut(new_child_id)
                .map(|new_child| {
                    new_child.parent = self.id;
                    new_child.next_sibling = first_child_id;
                });

            tree.sm
                .borrow_mut()
                .get_mut(first_child_id)
                .map(|first_child| {
                    first_child.prev_sibling = new_child_id;
                });

            tree.sm.borrow_mut().get_mut(self.id).map(|this_node| {
                if !this_node.children.1.is_null() {
                    this_node.children.0 = new_child_id;
                } else {
                    this_node.children.0 = new_child_id;
                    this_node.children.1 = new_child_id;
                }
            });

            tree.get(new_child_id)
        })
    }

    /// Inserts a new sibling before this node.
    pub fn insert_before(&mut self, value: T) -> Option<NodeRef<T>> {
        self.tree.upgrade().and_then(|tree| {
            let new_sibling_id = tree.orphan(value);
            self.insert_id_before(new_sibling_id)
        })
    }

    /// Inserts a sibling before this node.
    pub fn insert_id_before(&mut self, new_sibling_id: NodeId) -> Option<NodeRef<T>> {
        self.tree
            .upgrade()
            .zip(self.parent())
            .and_then(|(tree, parent)| {
                let prev_sibling_id = self
                    .prev_sibling()
                    .map(|prev_sibling| prev_sibling.id)
                    .unwrap_or_else(NodeId::null);

                tree.sm
                    .borrow_mut()
                    .get_mut(new_sibling_id)
                    .map(|new_sibling| {
                        new_sibling.parent = parent.id;
                        new_sibling.prev_sibling = prev_sibling_id;
                        new_sibling.next_sibling = self.id;
                    });

                tree.sm
                    .borrow_mut()
                    .get_mut(prev_sibling_id)
                    .map(|prev_sibling| {
                        prev_sibling.next_sibling = new_sibling_id;
                    });

                tree.sm.borrow_mut().get_mut(self.id).map(|this_node| {
                    this_node.prev_sibling = new_sibling_id;
                });

                tree.sm.borrow_mut().get_mut(parent.id).map(|parent| {
                    if parent.children.0 == self.id {
                        parent.children.0 = new_sibling_id;
                    }
                });

                tree.get(new_sibling_id)
            })
    }

    /// Inserts a new sibling after this node.
    pub fn insert_after(&mut self, value: T) -> Option<NodeRef<T>> {
        self.tree.upgrade().and_then(|tree| {
            let new_sibling_id = tree.orphan(value);
            self.insert_id_after(new_sibling_id)
        })
    }

    /// Inserts a sibling after this node.
    pub fn insert_id_after(&mut self, new_sibling_id: NodeId) -> Option<NodeRef<T>> {
        self.tree
            .upgrade()
            .zip(self.parent())
            .and_then(|(tree, parent)| {
                let next_sibling_id = self
                    .next_sibling()
                    .map(|next_sibling| next_sibling.id)
                    .unwrap_or_else(NodeId::null);

                tree.sm
                    .borrow_mut()
                    .get_mut(new_sibling_id)
                    .map(|new_sibling| {
                        new_sibling.parent = parent.id;
                        new_sibling.prev_sibling = self.id;
                        new_sibling.next_sibling = next_sibling_id;
                    });

                tree.sm
                    .borrow_mut()
                    .get_mut(next_sibling_id)
                    .map(|next_sibling| {
                        next_sibling.prev_sibling = new_sibling_id;
                    });

                tree.sm.borrow_mut().get_mut(self.id).map(|this_node| {
                    this_node.next_sibling = new_sibling_id;
                });

                tree.sm.borrow_mut().get_mut(parent.id).map(|parent| {
                    if parent.children.1 == self.id {
                        parent.children.1 = new_sibling_id;
                    }
                });

                tree.get(new_sibling_id)
            })
    }

    /// Reparents the children of a node, appending them to this node.
    pub fn reparent_from_id_append(&mut self, from_id: NodeId) {
        self.tree.upgrade().map(|tree| {
            let new_child_ids = tree
                .sm
                .borrow_mut()
                .get_mut(from_id)
                .map(|node| {
                    let new_child_ids = node.children;
                    node.children = (NodeId::null(), NodeId::null());
                    new_child_ids
                })
                .unwrap_or((NodeId::null(), NodeId::null()));

            if new_child_ids.0.is_null() && new_child_ids.1.is_null() {
                return;
            }

            tree.sm.borrow_mut().get_mut(new_child_ids.0).map(|node| {
                node.parent = self.id;
            });
            tree.sm.borrow_mut().get_mut(new_child_ids.1).map(|node| {
                node.parent = self.id;
            });

            let old_child_ids = tree.sm.borrow_mut().get_mut(self.id).and_then(|node| {
                if node.children.0.is_null() && node.children.1.is_null() {
                    node.children = new_child_ids;
                    None
                } else {
                    Some(node.children)
                }
            });
            let old_child_ids = match old_child_ids {
                Some(old_child_ids) => old_child_ids,
                None => return,
            };

            tree.sm.borrow_mut().get_mut(old_child_ids.1).map(|node| {
                node.next_sibling = new_child_ids.0;
            });
            tree.sm.borrow_mut().get_mut(new_child_ids.0).map(|node| {
                node.prev_sibling = old_child_ids.1;
            });

            tree.sm
                .borrow_mut()
                .get_mut(self.id)
                .map(|node| node.children = (old_child_ids.0, new_child_ids.1));
        });
    }
}

/// Creates a tree from expressions.
///
/// # Examples
///
/// ```
/// # use sws_tree::tree;
/// # fn main() {
/// let tree = tree!("root");
/// # }
/// ```
///
/// ```
/// # use sws_tree::tree;
/// # fn main() {
/// let tree = tree! {
///     "root" => {
///         "child a",
///         "child b" => {
///             "grandchild a",
///             "grandchild b",
///         },
///         "child c",
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! tree {
    (@ $n:ident { }) => { };

    // Last leaf.
    (@ $n:ident { $value:expr }) => {
        { $n.append($value).unwrap(); }
    };

    // Leaf.
    (@ $n:ident { $value:expr, $($tail:tt)* }) => {
        {
            $n.append($value).unwrap();
            tree!(@ $n { $($tail)* });
        }
    };

    // Last node with children.
    (@ $n:ident { $value:expr => $children:tt }) => {
        {
            let mut node = $n.append($value).unwrap();
            tree!(@ node $children);
        }
    };

    // Node with children.
    (@ $n:ident { $value:expr => $children:tt, $($tail:tt)* }) => {
        {
            {
                let mut node = $n.append($value).unwrap();
                tree!(@ node $children);
            }
            tree!(@ $n { $($tail)* });
        }
    };

    ($root:expr) => { $crate::Tree::new($root) };

    ($root:expr => $children:tt) => {
        {
            let tree = $crate::Tree::new($root);
            {
                let mut node = tree.root();
                tree!(@ node $children);
            }
            tree
        }
    };
}
