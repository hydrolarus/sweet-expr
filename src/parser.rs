use crate::lexer::Token;
use crate::value::*;
use std::{cmp::Ordering, iter::Peekable, ops::Range};

#[derive(Debug)]
pub enum ParseError<'src> {
    MismatchedToken {
        expected: Token<'static>,
        found: Token<'src>,
        span: Range<usize>,
    },
    ExpectedTokFoundEof {
        expected: Token<'static>,
        pos: Option<usize>,
    },
    ExpectedEofFoundToken {
        found: Token<'src>,
        span: Range<usize>,
    },
}

pub struct Parser<'src> {
    tokens: Vec<(Token<'src>, Range<usize>)>,
    cur_tok: usize,
}

impl<'src> Parser<'src> {
    pub fn new(tokens: impl Iterator<Item = (Token<'src>, Range<usize>)>) -> Self {
        Self {
            tokens: handle_whitespace(tokens),
            cur_tok: 0,
        }
    }

    fn advance(&mut self) {
        self.cur_tok += 1;
    }

    fn peek_tok(&self, n: usize) -> Option<(Token<'src>, Range<usize>)> {
        self.tokens.get(self.cur_tok + n).cloned()
    }

    fn last_tok_span(&self) -> Option<Range<usize>> {
        self.tokens.get(self.cur_tok - 1).map(|x| x.1.clone())
    }

    fn expect(
        &mut self,
        expected: Token<'static>,
    ) -> Result<(Token<'src>, Range<usize>), ParseError<'src>> {
        if let Some((tok, span)) = self.peek_tok(0) {
            if tok == expected {
                self.advance();
                Ok((tok, span))
            } else {
                Err(ParseError::MismatchedToken {
                    expected,
                    found: tok,
                    span,
                })
            }
        } else {
            let pos = if let Some((_, span)) = self.tokens.get(self.cur_tok - 1).cloned() {
                Some(span.end)
            } else {
                None
            };
            Err(ParseError::ExpectedTokFoundEof { expected, pos })
        }
    }

    pub fn parse_toplevel(&mut self) -> Result<Vec<Atom<'src>>, ParseError<'src>> {
        // the whole file might be indented
        let mut is_indented = false;
        if let Some((Token::Indent, _)) = self.peek_tok(0) {
            self.advance();
            is_indented = true;
        }

        // while there's any atom or indent-group, parse those
        let mut children = vec![];

        while self.atom_start() {
            children.push(self.parse_maybe_indent_group()?);
        }

        if let Some((Token::Dedent, _)) = self.peek_tok(0) {
            if is_indented {
                self.advance();
            }
        }
        if let Some((tok, span)) = self.peek_tok(0) {
            return Err(ParseError::ExpectedEofFoundToken { found: tok, span });
        }

        Ok(children)
    }

    pub fn parse_maybe_indent_group(&mut self) -> Result<Atom<'src>, ParseError<'src>> {
        let start_span = if let Some((_, span)) = self.peek_tok(0) {
            span
        } else {
            unreachable!()
        };
        let mut children = vec![];
        // first parse all n-atoms on the current line
        while self.atom_start() {
            children.push(self.parse_atom()?);
        }

        // if there's a newline + indent...
        //
        // this branch WILL return if successful so that
        // below it handles end-of-group for multiple cases
        if let Some((Token::Newline, _)) = self.peek_tok(0) {
            self.advance();
            if let Some((Token::Indent, _)) = self.peek_tok(0) {
                self.advance();

                // parse more indent groups

                while self.atom_start() {
                    children.push(self.parse_maybe_indent_group()?);

                    if let Some((Token::Dedent, _)) = self.peek_tok(0) {
                        break;
                    } else {
                        continue;
                    }
                }

                match self.peek_tok(0) {
                    Some((Token::Dedent, end_span)) => {
                        self.advance();
                        return Ok(Atom::Group(Group {
                            group_type: GroupType::Indentation,
                            children,
                            start_delim: Spanned((), start_span),
                            end_delim: Spanned((), end_span),
                        }));
                    }
                    Some((_, end_span)) => {
                        return Ok(Atom::Group(Group {
                            group_type: GroupType::Indentation,
                            children,
                            start_delim: Spanned((), start_span),
                            end_delim: Spanned((), end_span),
                        }))
                    }
                    None => {
                        // fall through to end-case
                    }
                };
            }
        }

        if children.len() == 1 {
            Ok(children.pop().unwrap())
        } else {
            // no new line, so end of file or error? stop here
            let end_span = self.last_tok_span().unwrap();
            Ok(Atom::Group(Group {
                group_type: GroupType::Indentation,
                children,
                start_delim: Spanned((), start_span),
                end_delim: Spanned((), end_span),
            }))
        }
    }

    pub fn parse_atom(&mut self) -> Result<Atom<'src>, ParseError<'src>> {
        let Some((tok, span)) = self.peek_tok(0) else {
            todo!()
        };
        match tok {
            Token::ParenOpen | Token::BracketOpen | Token::CurlyOpen => {
                let group = self.parse_explicit_group()?;
                Ok(Atom::Group(group))
            }
            Token::Identifier(ident) => {
                self.advance();
                let val = Atom::Identifier(Spanned(ident, span.clone()));

                if let Some((Token::ParenOpen | Token::BracketOpen | Token::CurlyOpen, next_span)) =
                    self.peek_tok(0)
                {
                    if next_span.start == span.end {
                        // neoteric expression
                        let group = self.parse_explicit_group()?;
                        Ok(Atom::Neoteric {
                            lhs: Box::new(val),
                            rhs: group,
                        })
                    } else {
                        Ok(val)
                    }
                } else {
                    Ok(val)
                }
            }
            Token::String(str) => {
                self.advance();
                Ok(Atom::String(Spanned(str, span)))
            }
            _ => todo!(),
        }
    }

    fn atom_start(&self) -> bool {
        let Some((tok, _span)) = self.peek_tok(0) else {
            return false;
        };
        matches!(
            tok,
            Token::ParenOpen
                | Token::BracketOpen
                | Token::CurlyOpen
                | Token::Identifier(_)
                | Token::String(_)
        )
    }

    fn parse_explicit_group(&mut self) -> Result<Group<'src>, ParseError<'src>> {
        let Some((tok, start_span)) = self.peek_tok(0) else {
            todo!()
        };

        self.advance();

        let mut children = vec![];
        while self.atom_start() {
            children.push(self.parse_atom()?);
        }

        let (to_expect, group_type) = match tok {
            Token::ParenOpen => (Token::ParenClose, GroupType::Parenthesis),
            Token::CurlyOpen => (Token::CurlyClose, GroupType::Curly),
            Token::BracketOpen => (Token::BracketClose, GroupType::Bracket),
            _ => unreachable!(),
        };

        let (_tok, end_span) = self.expect(to_expect)?;

        Ok(Group {
            group_type,
            start_delim: Spanned((), start_span),
            children,
            end_delim: Spanned((), end_span),
        })
    }
}

pub(crate) fn handle_whitespace<'src>(
    tokens: impl Iterator<Item = (Token<'src>, Range<usize>)>,
) -> Vec<(Token<'src>, Range<usize>)> {
    enum State {
        Start,
        StartOfLine,
        InLine,
        Ignore(usize),
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
                        toks.push((Token::Indent, span));
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
            State::StartOfLine => {
                if !matches!(tok, Token::Spaces(_) | Token::Newline | Token::Comment) {
                    // This could be a dedent too

                    if *indents.last().unwrap_or(&0) > 0 {
                        match pop_stack(&mut indents, 0) {
                            Ok(n) => {
                                for _ in 0..n {
                                    toks.push((Token::Dedent, span.clone()));
                                }
                            }
                            Err(_) => {
                                toks.push((Token::Error("Invalid indentation"), span.clone()))
                            }
                        }
                    }
                }
                match tok {
                    Token::Identifier(_) | Token::String(_) => {
                        toks.push((tok, span));
                        state = State::InLine;
                    }
                    Token::Comment => {
                        // comments are always followed by a new-line, so we just
                        // skip and let the newline handle the rest.
                        continue;
                    }
                    Token::ParenOpen | Token::CurlyOpen | Token::BracketOpen => {
                        toks.push((tok, span));
                        state = State::Ignore(1);
                    }
                    Token::ParenClose | Token::CurlyClose | Token::BracketClose => {
                        // weird to have them, let's just pass them to the parser
                        toks.push((tok, span));
                        state = State::InLine;
                    }

                    Token::Newline => {
                        // empty line! let's just skip this one
                        state = State::StartOfLine;
                        continue;
                    }
                    Token::Spaces(s) => {
                        // same as on a new line, check for indentation.
                        let indent = s.len();

                        if indent > *indents.last().unwrap_or(&0) {
                            indents.push(indent);
                            toks.push((Token::Indent, span));
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
                    Token::Error(_) | Token::Indent | Token::Dedent => {
                        toks.push((tok, span));
                        state = State::InLine;
                    }
                }
            }
            State::InLine => match tok {
                Token::Identifier(_) | Token::String(_) => {
                    toks.push((tok, span));
                    state = State::InLine;
                }
                Token::Comment => {
                    // comments are always followed by a new-line, so we just
                    // skip and let the newline handle the rest.
                    continue;
                }
                Token::ParenOpen | Token::CurlyOpen | Token::BracketOpen => {
                    toks.push((tok, span));
                    state = State::Ignore(1);
                }
                Token::ParenClose | Token::CurlyClose | Token::BracketClose => {
                    toks.push((tok, span));
                    continue;
                }
                Token::Newline => {
                    toks.push((tok, span));
                    state = State::StartOfLine;
                }
                Token::Spaces(_) => {
                    // inside a line, whitespace is not significant
                    continue;
                }
                Token::Error(_) | Token::Indent | Token::Dedent => {
                    toks.push((tok, span));
                }
            },
            State::Ignore(n) => match tok {
                Token::Identifier(_) | Token::String(_) => {
                    toks.push((tok, span));
                    continue;
                }
                Token::Comment => {
                    continue;
                }
                Token::ParenOpen | Token::CurlyOpen | Token::BracketOpen => {
                    toks.push((tok, span));
                    state = State::Ignore(n + 1);
                }
                Token::ParenClose | Token::CurlyClose | Token::BracketClose => {
                    toks.push((tok, span));
                    if n == 1 {
                        state = State::InLine;
                    } else {
                        state = State::Ignore(n - 1);
                    }
                }
                Token::Newline | Token::Spaces(_) => {
                    continue;
                }
                Token::Error(_) | Token::Indent | Token::Dedent => {
                    toks.push((tok, span));
                }
            },
        }
    }

    toks
}
