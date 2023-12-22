//! HTML parsing and querying with CSS selectors.
//!
//! Partial port of [scraper][] using [sws-tree][] which uses [`Rc`](std::rc::Rc)
//! instead of references with lifetimes.
//!
//! [scraper]: https://crates.io/crates/scraper
//! [sws-tree]: https://crates.io/crates/sws-tree

#[macro_use]
extern crate html5ever;

pub mod element_ref;
pub mod error;
pub mod html;
pub mod node;
pub mod selector;

pub use crate::element_ref::ElementRef;
pub use crate::html::Html;
pub use crate::node::Node;
pub use crate::selector::Selector;

pub use selectors::{attr::CaseSensitivity, Element};
pub use tendril::StrTendril;
