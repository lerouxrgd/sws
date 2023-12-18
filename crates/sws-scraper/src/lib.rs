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
