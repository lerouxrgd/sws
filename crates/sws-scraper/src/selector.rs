//! CSS selectors.

use std::convert::TryFrom;
use std::fmt;

use selectors::parser::SelectorParseErrorKind;
use selectors::{matching, parser};
use smallvec::SmallVec;
use string_cache::{Atom, EmptyStaticAtomSet};

use crate::atoms::{AtomIdent, AtomString, WeakAtom};
use crate::element_ref::ElementRef;

/// Wrapper around CSS selectors.
///
/// Represents a "selector group", i.e. a comma-separated list of selectors.
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    /// The CSS selectors.
    pub selectors: SmallVec<[parser::Selector<Simple>; 1]>,
}

impl Selector {
    /// Parses a CSS selector group.
    pub fn parse(
        selectors: &'_ str,
    ) -> Result<Self, cssparser::ParseError<'_, SelectorParseErrorKind<'_>>> {
        let mut parser_input = cssparser::ParserInput::new(selectors);
        let mut parser = cssparser::Parser::new(&mut parser_input);
        parser::SelectorList::parse(&Parser, &mut parser).map(|list| Selector { selectors: list.0 })
    }

    /// Returns true if the element matches this selector.
    pub fn matches(&self, element: &ElementRef) -> bool {
        self.matches_with_scope(element, None)
    }

    /// Returns true if the element matches this selector.
    /// The optional `scope` argument is used to specify which element has `:scope` pseudo-class.
    /// When it is `None`, `:scope` will match the root element.
    pub fn matches_with_scope(&self, element: &ElementRef, scope: Option<ElementRef>) -> bool {
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            None,
            matching::QuirksMode::NoQuirks,
        );
        context.scope_element = scope.map(|x| selectors::Element::opaque(&x));
        self.selectors
            .iter()
            .any(|s| matching::matches_selector(s, 0, None, element, &mut context, &mut |_, _| {}))
    }
}

impl<'i> TryFrom<&'i str> for Selector {
    type Error = cssparser::ParseError<'i, SelectorParseErrorKind<'i>>;

    fn try_from(s: &'i str) -> Result<Self, Self::Error> {
        Selector::parse(s)
    }
}

////////////////////////////////////////////////////////////////////////////////////////

// An implementation of `Parser` for `selectors`
struct Parser;

impl<'i> parser::Parser<'i> for Parser {
    type Impl = Simple;
    type Error = SelectorParseErrorKind<'i>;
}

////////////////////////////////////////////////////////////////////////////////////////

/// A simple implementation of `SelectorImpl` with no pseudo-classes or pseudo-elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Simple;

impl parser::SelectorImpl for Simple {
    type ExtraMatchingData = InvalidationMatchingData;
    type AttrValue = AtomString;
    type Identifier = AtomIdent;
    type LocalName = AtomIdent;
    type NamespacePrefix = AtomIdent;
    type NamespaceUrl = Namespace;
    type BorrowedNamespaceUrl = Namespace;
    type BorrowedLocalName = WeakAtom;

    type PseudoElement = PseudoElement;
    type NonTSPseudoClass = NonTSPseudoClass;
}

////////////////////////////////////////////////////////////////////////////////////////

/// Non Tree-Structural Pseudo-Class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonTSPseudoClass {}

impl parser::NonTSPseudoClass for NonTSPseudoClass {
    type Impl = Simple;

    fn is_active_or_hover(&self) -> bool {
        false
    }

    fn is_user_action_state(&self) -> bool {
        false
    }
}

impl cssparser::ToCss for NonTSPseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str("")
    }
}

////////////////////////////////////////////////////////////////////////////////////////

/// CSS Pseudo-Element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoElement {}

impl parser::PseudoElement for PseudoElement {
    type Impl = Simple;
}

impl cssparser::ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str("")
    }
}

////////////////////////////////////////////////////////////////////////////////////////

/// A struct holding the members necessary to invalidate document state
/// selectors.
pub struct InvalidationMatchingData {
    /// The document state that has changed, which makes it always match.
    pub document_state: DocumentState,
}

impl Default for InvalidationMatchingData {
    #[inline(always)]
    fn default() -> Self {
        Self {
            document_state: DocumentState::empty(),
        }
    }
}

bitflags::bitflags! {
    /// Event-based document states.
    pub struct DocumentState: u64 {
        /// RTL locale: specific to the XUL localedir attribute
        const NS_DOCUMENT_STATE_RTL_LOCALE = 1 << 0;
        /// Window activation status
        const NS_DOCUMENT_STATE_WINDOW_INACTIVE = 1 << 1;
    }
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Namespace(pub Atom<EmptyStaticAtomSet>);

impl std::ops::Deref for Namespace {
    type Target = Atom<EmptyStaticAtomSet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn selector_conversions() {
        let s = "#testid.testclass";
        let _sel: Selector = s.try_into().unwrap();

        let s = s.to_owned();
        let _sel: Selector = (*s).try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_selector_conversions() {
        let s = "<failing selector>";
        let _sel: Selector = s.try_into().unwrap();
    }
}
