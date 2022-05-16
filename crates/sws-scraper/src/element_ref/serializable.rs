use std::io::Error;

use html5ever::serialize::{Serialize, Serializer, TraversalScope};
use sws_tree::iter::Edge;

use crate::element_ref::ElementRef;
use crate::node::Node;

impl Serialize for ElementRef {
    fn serialize<S>(&self, serializer: &mut S, traversal_scope: TraversalScope) -> Result<(), Error>
    where
        S: Serializer,
    {
        for edge in self.traverse() {
            match edge {
                Edge::Open(node) => {
                    if node == **self && traversal_scope == TraversalScope::ChildrenOnly(None) {
                        continue;
                    }

                    node.map_value(|v| match *v {
                        Node::Doctype(ref doctype) => serializer.write_doctype(doctype.name()),
                        Node::Comment(ref comment) => serializer.write_comment(comment),
                        Node::Text(ref text) => serializer.write_text(text),
                        Node::Element(ref elem) => {
                            let attrs = elem.attrs.iter().map(|(k, v)| (k, &v[..]));
                            serializer.start_elem(elem.name.clone(), attrs)
                        }
                        _ => Ok(()),
                    })
                    .transpose()?;
                }

                Edge::Close(node) => {
                    if node == **self && traversal_scope == TraversalScope::ChildrenOnly(None) {
                        continue;
                    }

                    node.map_value(|v| {
                        if let Some(elem) = v.as_element() {
                            serializer.end_elem(elem.name.clone())
                        } else {
                            Ok(())
                        }
                    })
                    .transpose()?;
                }
            }
        }

        Ok(())
    }
}
