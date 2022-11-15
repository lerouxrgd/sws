use std::fmt::{self, Write};

use string_cache::DefaultAtom;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct AtomString(pub DefaultAtom);

impl AsRef<str> for AtomString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'a> From<&'a str> for AtomString {
    #[inline]
    fn from(string: &str) -> Self {
        Self(DefaultAtom::from(string))
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
