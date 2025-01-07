use std::ops::Range;

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone, Copy)]
pub enum Token<'src> {
    #[regex(r#"[^\s\(\)\{\}\[\]\";]+"#)]
    Identifier(&'src str),

    #[regex(r#""([^"\\]|\\")*""#)]
    String(&'src str),

    #[regex(r";[^\\n]*")]
    Comment,

    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token("{")]
    CurlyOpen,
    #[token("}")]
    CurlyClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,

    #[regex("[ \\t\\f]*\n")]
    #[regex("[ \\t\\f]*\r\n")]
    Newline,
    #[regex(r"[ \t\f]+")]
    Spaces(&'src str),

    Error(&'src str),

    // These are only inserted later by the parser when whitespace and comments
    // are stripped
    Indent,
    Dedent,
}

pub fn tokenise(s: &'_ str) -> impl Iterator<Item = (Token<'_>, Range<usize>)> {
    Token::lexer(s)
        .spanned()
        .map(|(tok, span)| (tok.unwrap_or(Token::Error("Invalid token")), span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn some_test() {
        let lexer =
            tokenise("hello-world 13 (a b) (({a + 12.f}))[1] \"hello \\\"world\\\"\"\n   hello\n  äußerst entzückend");

        for (token, span) in lexer {
            dbg!(token, span);
        }
    }
}
