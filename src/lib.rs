pub mod lexer;
pub mod parser;
pub mod value;

#[cfg(test)]
mod tests {
    use lexer::tokenise;
    use parser::{handle_whitespace, Parser};

    use super::*;

    #[test]
    fn it_works() {
        let source = r#"
define test (a b)
    (print "hello")
    (another-thing 1 2)

test{1 + 3}

(test {1 + 3})"#;

        let toks = tokenise(source);

        dbg!(handle_whitespace(tokenise(source)));
        let mut parser = Parser::new(toks);

        let toplevel = parser.parse_toplevel();

        dbg!(toplevel);
        panic!()
    }
}
