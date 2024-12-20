use crate::lexer::Token;
use crate::value::*;
use std::{cmp::Ordering, iter::Peekable, ops::Range};

pub enum ParseError {}

enum Indentation {
    NoChange,
    Indent(usize),
    Dedent(usize),
}

pub struct Parser<TokenIter: Iterator> {
    tokens: Peekable<TokenIter>,
    indent_stack: Vec<usize>,
    indent_ignore_level: usize,
}

impl<'src, I: Iterator<Item = (Token<'src>, Range<usize>)>> Parser<I> {
    pub fn new(tokens: I) -> Self {
        Self {
            tokens: tokens.peekable(),
            indent_stack: vec![],
            indent_ignore_level: 0,
        }
    }

    pub fn parse_atom(&mut self) -> Result<Atom<'src>, ParseError> {
        self.skip_whitespace();

        let Some((tok, span)) = self.tokens.peek().cloned() else {
            todo!()
        };
        match tok {
            Token::ParenOpen | Token::BracketOpen | Token::CurlyOpen => {
                let group = self.parse_group()?;
                Ok(Atom::Group(group))
            }
            Token::Identifier(ident) => {
                _ = self.tokens.next();
                Ok(Atom::Identifier(Spanned(ident, span)))
            }
            Token::String(str) => {
                _ = self.tokens.next();
                Ok(Atom::String(Spanned(str, span)))
            }
            _ => todo!(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some((tok, _)) = self.tokens.peek() {
            match tok {
                Token::Comment => {}
                Token::Newline => {}
                Token::Spaces(_) => {}
                _ => {}
            }
        }
    }
    fn skip_whitespace_new_line(&mut self) {
        while let Some((tok, _)) = self.tokens.peek() {
            match tok {
                Token::Spaces(s) => {
                    let indent = s.len();
                    _ = self.tokens.next();
                }
                Token::Comment => {}
                Token::Newline => {}

                _ => {}
            }
        }
    }

    fn parse_group(&mut self) -> Result<Group<'src>, ParseError> {
        todo!()
    }
}

fn handle_whitespace<'src>(
    tokens: impl Iterator<Item = (Token<'src>, Range<usize>)>,
) -> Vec<(Token<'src>, Range<usize>)> {
    enum State {
        Start,
        InLine,
        Ignore(usize),
        StartOfLine,
    }

    let mut indents = vec![];

    let mut toks = vec![];

    let mut state = State::Start;

    fn pop_stack(stack: &mut Vec<usize>, level: usize) -> Result<usize, ()> {
        let mut levels_to_pop = 0;
        loop {
            let last_indent = *stack.last().unwrap_or(&0);
            match last_indent.cmp(&level) {
                Ordering::Less => return Err(()),
                Ordering::Equal => return Ok(levels_to_pop),
                Ordering::Greater => {
                    levels_to_pop += 1;
                    _ = stack.pop();
                }
            }
        }
    }

    for (tok, span) in tokens {
        match state {
            State::Start => match tok {
                Token::Identifier(_) | Token::String(_) => {
                    toks.push((tok, span));
                    state = State::InLine;
                }
                Token::Comment => {
                    // comment can only be followed by a newline or
                    // the end of the file, so we just skip it and don't change
                    // the state.
                    continue;
                }
                Token::Spaces(s) => {
                    // same as on a new line, check for indentation.
                    let indent = s.len();

                    if indent > *indents.last().unwrap_or(&0) {
                        indents.push(indent);
                    } else {
                        match pop_stack(&mut indents, indent) {
                            Ok(n) => {
                                for _ in 0..n {
                                    toks.push((Token::Dedent, span.clone()));
                                }
                            }
                            Err(_) => toks.push((Token::Error("Invalid indentation"), span)),
                        }
                    }

                    state = State::InLine;
                }

                Token::ParenOpen | Token::CurlyOpen | Token::BracketOpen => {
                    toks.push((tok, span));
                    state = State::Ignore(1);
                }
                Token::ParenClose | Token::CurlyClose | Token::BracketClose => {
                    // weird to start with those, but let's just let the parser
                    // handle it
                    toks.push((tok, span));
                    state = State::InLine;
                }
                Token::Newline => {
                    // starting with a new-line is like as if the next line is
                    // the start
                    continue;
                }
                Token::Error(_) | Token::Indent | Token::Dedent => {
                    // all else, pass through
                    toks.push((tok, span));
                }
            },
            State::InLine => todo!(),
            State::Ignore(_) => todo!(),
            State::StartOfLine => todo!(),
        }
    }

    todo!()
}
