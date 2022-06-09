use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching;
use selectors::{Element, OpaqueElement};

use crate::atoms::AtomString;
use crate::element_ref::ElementRef;
use crate::selector::{NonTSPseudoClass, PseudoElement, Simple};

/// Note: will never match against non-tree-structure pseudo-classes.
impl Element for ElementRef {
    type Impl = Simple;

    fn opaque(&self) -> OpaqueElement {
        self.node
            .map_value(|v| OpaqueElement::new(v))
            .expect("ElementRef isn't valid (null Element)")
    }

    fn parent_element(&self) -> Option<Self> {
        self.parent().and_then(ElementRef::wrap)
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn is_part(&self, _name: &AtomString) -> bool {
        false
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.map_value(|v1| other.map_value(|v2| v1.name == v2.name))
            .unwrap_or(Some(false))
            .unwrap_or(false)
    }

    fn imported_part(&self, _: &AtomString) -> Option<AtomString> {
        None
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.prev_siblings()
            .find(|sibling| sibling.map_value(|v| v.is_element()).unwrap_or(false))
            .map(ElementRef::new)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.next_siblings()
            .find(|sibling| sibling.map_value(|v| v.is_element()).unwrap_or(false))
            .map(ElementRef::new)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        // FIXME: Is there more to this?
        self.map_value(|v| v.name.ns == ns!(html)).unwrap_or(false)
    }

    fn has_local_name(&self, name: &AtomString) -> bool {
        self.map_value(|v| v.name.local.as_ref() == name.as_ref())
            .unwrap_or(false)
    }

    fn has_namespace(&self, namespace: &AtomString) -> bool {
        self.map_value(|v| v.name.ns.as_ref() == namespace.as_ref())
            .unwrap_or(false)
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&AtomString>,
        local_name: &AtomString,
        operation: &AttrSelectorOperation<&AtomString>,
    ) -> bool {
        self.map_value(|v| {
            v.attrs.iter().any(|(key, value)| {
                !matches!(*ns, NamespaceConstraint::Specific(url) if url.as_ref() != key.ns.as_ref())
                    && local_name.as_ref() == key.local.as_ref()
                    && operation.eval_str(value)
            })
        })
        .unwrap_or(false)
    }

    fn match_non_ts_pseudo_class<F>(
        &self,
        _pc: &NonTSPseudoClass,
        _context: &mut matching::MatchingContext<Self::Impl>,
        _flags_setter: &mut F,
    ) -> bool {
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _context: &mut matching::MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn is_link(&self) -> bool {
        self.map_value(|v| v.name() == "link").unwrap_or(false)
    }

    fn is_html_slot_element(&self) -> bool {
        true
    }

    fn has_id(&self, id: &AtomString, case_sensitivity: CaseSensitivity) -> bool {
        self.map_value(|v| match v.id {
            Some(ref val) => case_sensitivity.eq(id.as_ref().as_bytes(), val.as_bytes()),
            None => false,
        })
        .unwrap_or(false)
    }

    fn has_class(&self, name: &AtomString, case_sensitivity: CaseSensitivity) -> bool {
        self.map_value(|v| v.has_class(name.as_ref(), case_sensitivity))
            .unwrap_or(false)
    }

    fn is_empty(&self) -> bool {
        !self.children().any(|child| {
            child.map_value(|v| v.is_element()).unwrap_or(false)
                || child.map_value(|v| v.is_text()).unwrap_or(false)
        })
    }

    fn is_root(&self) -> bool {
        self.parent().map_or(false, |parent| {
            parent.map_value(|v| v.is_document()).unwrap_or(false)
        })
    }
}

#[cfg(test)]
mod tests {
    use selectors::attr::CaseSensitivity;
    use selectors::Element;

    use crate::html::Html;
    use crate::selector::Selector;

    #[test]
    fn test_has_id() {
        use crate::atoms::AtomString;

        let html = "<p id='link_id_456'>hey there</p>";
        let fragment = Html::parse_fragment(html);
        let sel = Selector::parse("p").unwrap();

        let element = fragment.select(sel.clone()).next().unwrap();
        assert_eq!(
            true,
            element.has_id(
                &AtomString::from("link_id_456"),
                CaseSensitivity::CaseSensitive
            )
        );

        let html = "<p>hey there</p>";
        let fragment = Html::parse_fragment(html);
        let element = fragment.select(sel).next().unwrap();
        assert_eq!(
            false,
            element.has_id(
                &AtomString::from("any_link_id"),
                CaseSensitivity::CaseSensitive
            )
        );
    }

    #[test]
    fn test_is_link() {
        let html = "<link href='https://www.example.com'>";
        let fragment = Html::parse_fragment(html);
        let sel = Selector::parse("link").unwrap();
        let element = fragment.select(sel).next().unwrap();
        assert_eq!(true, element.is_link());

        let html = "<p>hey there</p>";
        let fragment = Html::parse_fragment(html);
        let sel = Selector::parse("p").unwrap();
        let element = fragment.select(sel).next().unwrap();
        assert_eq!(false, element.is_link());
    }

    #[test]
    fn test_has_class() {
        use crate::atoms::AtomString;

        let html = "<p class='my_class'>hey there</p>";
        let fragment = Html::parse_fragment(html);
        let sel = Selector::parse("p").unwrap();
        let element = fragment.select(sel).next().unwrap();
        assert_eq!(
            true,
            element.has_class(
                &AtomString::from("my_class"),
                CaseSensitivity::CaseSensitive
            )
        );

        let html = "<p>hey there</p>";
        let fragment = Html::parse_fragment(html);
        let sel = Selector::parse("p").unwrap();
        let element = fragment.select(sel).next().unwrap();
        assert_eq!(
            false,
            element.has_class(
                &AtomString::from("my_class"),
                CaseSensitivity::CaseSensitive
            )
        );
    }
}
