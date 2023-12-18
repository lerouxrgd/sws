//! Element references.

mod element;
mod serializable;

use std::iter::FusedIterator;
use std::ops::Deref;

use html5ever::serialize::{serialize, SerializeOpts, TraversalScope};
use sws_tree::iter::{Edge, Traverse};
use sws_tree::NodeRef;

use crate::node::{Element, Node};
use crate::selector::Selector;

/// Wrapper around a reference to an element node.
///
/// This wrapper implements the `Element` trait from the `selectors` crate, which allows
/// it to be matched against CSS selectors.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementRef {
    node: NodeRef<Node>,
}

impl ElementRef {
    fn new(node: NodeRef<Node>) -> Self {
        ElementRef { node }
    }

    /// Wraps a `NodeRef` only if it references a `Node::Element`.
    pub fn wrap(node: NodeRef<Node>) -> Option<Self> {
        match node.map_value(|v| v.is_element()) {
            Some(true) => Some(ElementRef::new(node)),
            _ => None,
        }
    }

    /// Maps a function to the `Element` referenced by `self`.
    pub fn map_value<F, R>(&self, map_fn: F) -> Option<R>
    where
        F: FnOnce(&Element) -> R,
    {
        self.node.map_value(|v| map_fn(v.as_element().unwrap()))
    }

    /// Returns an iterator over descendent elements matching a selector.
    pub fn select(&self, selector: Selector) -> Select {
        let mut inner = self.traverse();
        inner.next(); // Skip Edge::Open(self).

        Select {
            scope: self.clone(),
            inner,
            selector,
        }
    }

    fn serialize(&self, traversal_scope: TraversalScope) -> String {
        let opts = SerializeOpts {
            scripting_enabled: false, // It's not clear what this does.
            traversal_scope,
            create_missing_parent: false,
        };
        let mut buf = Vec::new();
        serialize(&mut buf, self, opts).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// Returns the HTML of this element.
    pub fn html(&self) -> String {
        self.serialize(TraversalScope::IncludeNode)
    }

    /// Returns the inner HTML of this element.
    pub fn inner_html(&self) -> String {
        self.serialize(TraversalScope::ChildrenOnly(None))
    }

    /// Returns an iterator over descendent text nodes.
    pub fn text(&self) -> Text {
        Text {
            inner: self.traverse(),
        }
    }

    /// Returns all the descendent text nodes content concatenated.
    pub fn inner_text(&self) -> String {
        let mut all_text = String::new();
        for edge in self.traverse() {
            if let Edge::Open(node) = edge {
                node.map_value(|v| {
                    if let Node::Text(ref text) = v {
                        all_text.push_str(text);
                    }
                });
            }
        }
        all_text
    }
}

impl Deref for ElementRef {
    type Target = NodeRef<Node>;

    fn deref(&self) -> &NodeRef<Node> {
        &self.node
    }
}

/// Iterator over descendent elements matching a selector.
#[derive(Debug, Clone)]
pub struct Select {
    scope: ElementRef,
    inner: Traverse<Node>,
    selector: Selector,
}

impl Iterator for Select {
    type Item = ElementRef;

    fn next(&mut self) -> Option<ElementRef> {
        for edge in &mut self.inner {
            if let Edge::Open(node) = edge {
                if let Some(element) = ElementRef::wrap(node) {
                    if self
                        .selector
                        .matches_with_scope(&element, Some(self.scope.clone()))
                    {
                        return Some(element);
                    }
                }
            }
        }
        None
    }
}

impl FusedIterator for Select {}

/// Iterator over descendent text nodes.
#[derive(Debug, Clone)]
pub struct Text {
    inner: Traverse<Node>,
}

impl Iterator for Text {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        for edge in &mut self.inner {
            if let Edge::Open(node) = edge {
                let text = node
                    .map_value(|v| {
                        if let Node::Text(ref text) = v {
                            Some(text.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap();
                if text.is_some() {
                    return text;
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::html::Html;
    use crate::selector::Selector;

    #[test]
    fn test_scope() {
        let html = r"
            <div>
                <b>1</b>
                <span>
                    <span><b>2</b></span>
                    <b>3</b>
                </span>
            </div>
        ";
        let fragment = Html::parse_fragment(html);
        let sel1 = Selector::parse("div > span").unwrap();
        let sel2 = Selector::parse(":scope > b").unwrap();

        let element1 = fragment.select(sel1).next().unwrap();
        let element2 = element1.select(sel2).next().unwrap();
        assert_eq!(element2.inner_html(), "3");
    }
}
