//! HTML documents and fragments.

mod tree_sink;

use std::borrow::Cow;
use std::rc::Rc;

use html5ever::driver;
use html5ever::tree_builder::QuirksMode;
use html5ever::QualName;
use sws_tree::Tree;
use tendril::TendrilSink;

use crate::element_ref::{ElementRef, Select};
use crate::node::Node;
use crate::selector::Selector;

/// An HTML tree.
///
/// Parsing does not fail hard. Instead, the `quirks_mode` is set and errors are added to the
/// `errors` field. The `tree` will still be populated as best as possible.
///
/// Implements the `TreeSink` trait from the `html5ever` crate, which allows HTML to be parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct Html {
    /// Parse errors.
    pub errors: Vec<Cow<'static, str>>,

    /// The quirks mode.
    pub quirks_mode: QuirksMode,

    /// The node tree.
    pub tree: Rc<Tree<Node>>,
}

impl Html {
    /// Creates an empty HTML document.
    pub fn new_document() -> Self {
        Html {
            errors: Vec::new(),
            quirks_mode: QuirksMode::NoQuirks,
            tree: Tree::new(Node::Document),
        }
    }

    /// Creates an empty HTML fragment.
    pub fn new_fragment() -> Self {
        Html {
            errors: Vec::new(),
            quirks_mode: QuirksMode::NoQuirks,
            tree: Tree::new(Node::Fragment),
        }
    }

    /// Parses a string of HTML as a document.
    ///
    /// This is a convenience method for the following:
    ///
    /// ```
    /// # extern crate html5ever;
    /// # extern crate sws_scraper;
    /// # extern crate tendril;
    /// # fn main() {
    /// # let document = "";
    /// use html5ever::driver::{self, ParseOpts};
    /// use sws_scraper::Html;
    /// use tendril::TendrilSink;
    ///
    /// let parser = driver::parse_document(Html::new_document(), ParseOpts::default());
    /// let html = parser.one(document);
    /// # }
    /// ```
    pub fn parse_document(document: &str) -> Self {
        let parser = driver::parse_document(Self::new_document(), Default::default());
        parser.one(document)
    }

    /// Parses a string of HTML as a fragment.
    pub fn parse_fragment(fragment: &str) -> Self {
        let parser = driver::parse_fragment(
            Self::new_fragment(),
            Default::default(),
            QualName::new(None, ns!(html), local_name!("body")),
            Vec::new(),
        );
        parser.one(fragment)
    }

    /// Returns an iterator over elements matching a selector.
    pub fn select(&self, selector: Selector) -> Select {
        self.root_element().select(selector)
    }

    /// Returns the root `<html>` element.
    pub fn root_element(&self) -> ElementRef {
        let root_node = self
            .tree
            .root()
            .children()
            .find(|child| child.map_value(|v| v.is_element()).unwrap_or(false))
            .expect("html node missing");
        ElementRef::wrap(root_node).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::Html;
    use super::Selector;

    #[test]
    fn root_element_fragment() {
        let html = Html::parse_fragment(r#"<a href="http://github.com">1</a>"#);
        let root_ref = html.root_element();
        let href = root_ref
            .select(Selector::parse("a").unwrap())
            .next()
            .unwrap();
        assert_eq!(href.inner_html(), "1");
        assert_eq!(
            href.map_value(|v| v.attr("href").unwrap().to_string())
                .unwrap(),
            "http://github.com"
        );
    }

    #[test]
    fn root_element_document_doctype() {
        let html = Html::parse_document("<!DOCTYPE html>\n<title>abc</title>");
        let root_ref = html.root_element();
        let title = root_ref
            .select(Selector::parse("title").unwrap())
            .next()
            .unwrap();
        assert_eq!(title.inner_html(), "abc");
    }

    #[test]
    fn root_element_document_comment() {
        let html = Html::parse_document("<!-- comment --><title>abc</title>");
        let root_ref = html.root_element();
        let title = root_ref
            .select(Selector::parse("title").unwrap())
            .next()
            .unwrap();
        assert_eq!(title.inner_html(), "abc");
    }
}
