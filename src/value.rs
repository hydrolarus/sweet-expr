use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct Spanned<T>(pub T, pub std::ops::Range<usize>);

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub enum Atom<'src> {
    /// Any non-string,-bracket or -whitespace sequence of characters
    Identifier(Spanned<&'src str>),
    /// A sequence of characters between two " chars, only checks for \" escapes
    String(Spanned<&'src str>),
    Group(Group<'src>),
    Neoteric {
        lhs: Box<Atom<'src>>,
        rhs: Group<'src>,
    },
}

#[derive(Debug)]
pub enum GroupType {
    Indentation, // practically the same as Parenthesis
    Parenthesis,
    Curly,
    Bracket,
}

#[derive(Debug)]
pub struct Group<'src> {
    pub group_type: GroupType,
    pub start_delim: Spanned<()>,
    pub children: Vec<Atom<'src>>,
    pub end_delim: Spanned<()>,
}
