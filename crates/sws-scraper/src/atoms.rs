use std::fmt::{self, Write};

use string_cache::{Atom, EmptyStaticAtomSet};

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct AtomString(pub Atom<EmptyStaticAtomSet>);

impl AsRef<str> for AtomString {
    fn as_ref(&self) -> &str {
        &*self.0
    }
}

impl<'a> From<&'a str> for AtomString {
    #[inline]
    fn from(string: &str) -> Self {
        Self(Atom::from(string))
    }
}

impl cssparser::ToCss for AtomString {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: Write,
    {
        cssparser::CssStringWriter::new(dest).write_str(self.as_ref())
    }
}

////////////////////////////////////////////////////////////////////////////////////////

pub struct GenericAtomIdent<Set>(pub string_cache::Atom<Set>)
where
    Set: string_cache::StaticAtomSet;

pub type AtomIdent = GenericAtomIdent<EmptyStaticAtomSet>;

impl<Set: string_cache::StaticAtomSet> std::fmt::Debug for GenericAtomIdent<Set> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<Set: string_cache::StaticAtomSet> Clone for GenericAtomIdent<Set> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Set: string_cache::StaticAtomSet> Default for GenericAtomIdent<Set> {
    fn default() -> Self {
        Self(string_cache::Atom::default())
    }
}

impl<Set: string_cache::StaticAtomSet> Eq for GenericAtomIdent<Set> {}

impl<Set: string_cache::StaticAtomSet> PartialEq for GenericAtomIdent<Set> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Set: string_cache::StaticAtomSet> std::hash::Hash for GenericAtomIdent<Set> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<'a, Set: string_cache::StaticAtomSet> From<&'a str> for GenericAtomIdent<Set> {
    #[inline]
    fn from(string: &str) -> Self {
        Self(string_cache::Atom::from(string))
    }
}

impl<Set: string_cache::StaticAtomSet> std::borrow::Borrow<string_cache::Atom<Set>>
    for GenericAtomIdent<Set>
{
    #[inline]
    fn borrow(&self) -> &string_cache::Atom<Set> {
        &self.0
    }
}

impl<Set: string_cache::StaticAtomSet> std::ops::Deref for GenericAtomIdent<Set> {
    type Target = Atom<Set>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Set: string_cache::StaticAtomSet> cssparser::ToCss for GenericAtomIdent<Set> {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: Write,
    {
        serialize_atom_identifier(&self.0, dest)
    }
}

pub fn serialize_atom_identifier<Static, W>(
    ident: &::string_cache::Atom<Static>,
    dest: &mut W,
) -> fmt::Result
where
    Static: string_cache::StaticAtomSet,
    W: Write,
{
    cssparser::serialize_identifier(&ident, dest)
}

////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WeakAtom(Atom<EmptyStaticAtomSet>);

impl std::borrow::Borrow<WeakAtom> for GenericAtomIdent<EmptyStaticAtomSet> {
    #[inline]
    fn borrow(&self) -> &WeakAtom {
        unsafe { std::mem::transmute(&WeakAtom(self.0.clone())) }
    }
}

impl std::ops::Deref for WeakAtom {
    type Target = Atom<EmptyStaticAtomSet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
