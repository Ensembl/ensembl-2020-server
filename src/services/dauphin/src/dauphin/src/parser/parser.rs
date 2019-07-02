use crate::lexer::{ Lexer, Token };

use super::node::Statement;
use super::preproc::preprocess;

/* https://steinwaywu.ghost.io/operator-precedence-climbing-parsing-with-user-defined-operators/ */

struct Parser {
    lexer: Lexer    
}

fn is_reserved(s: &str) -> bool {
    s == "reserved"
}

impl Parser {
    fn new(lexer: Lexer) -> Parser {
        Parser {
            lexer
        }
    }

    fn parse_import(&mut self) -> Statement {
        self.lexer.get();
        if let Token::LiteralString(loc) = self.lexer.get() {
            Statement::Import(loc)
        } else {
            Statement::Error("bad import statement".to_string())
        }
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        let token = self.lexer.peek();
        print!("token: {:?}\n",token);
        if let Token::Identifier(id) = token {
            let out = match &id[..] {
                "import" => Some(self.parse_import()),
                x => if is_reserved(x) {
                    Some(Statement::Error(format!("Reserved keyword '{}' found",x)))
                } else {
                    Some(Statement::Error("TODO".to_string()))
                }
            };
            if out.is_none() {
                return out;
            }
            if let Some(Statement::Error(_)) = out {
                return out;
            }
            if out.is_some() && self.lexer.get() != Token::Other(';') {
                Some(Statement::Error("Unterminated statement".to_string()))
            } else {
                out
            }
        } else {
            Some(Statement::Error("TODO".to_string()))
        }
    }

    fn try_get_preprocessed_statement(&mut self) -> Option<Statement> {
        if let Some(stmt) = self.parse_statement() {
            match preprocess(&stmt,&mut self.lexer) {
                Ok(true) => None,  /* preprocessed, so consumed */
                Ok(false) => Some(stmt), /* not preprocessed, so still exists */
                Err(error) => Some(Statement::Error(error))
            }
        } else {
            None
        }
    }

    fn get_preprocessed_statement(&mut self) -> Statement {
        loop {
            if let Some(stmt) = self.try_get_preprocessed_statement() {
                return stmt;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::preproc::preprocess;
    use crate::lexer::FileResolver;

    #[test]
    fn statement() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("data: import \"x\";").ok();
        let mut p = Parser::new(lexer);
        assert_eq!(Some(Statement::Import("x".to_string())),p.parse_statement());
    }

    #[test]
    fn import_statement() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("data: import \"data: *;\";").ok();
        let mut p = Parser::new(lexer);
        preprocess(&p.parse_statement().unwrap(),&mut p.lexer).expect("100");
        let tok = p.lexer.get().clone();
        assert_eq!(Token::Other('*'),tok);
        assert_eq!("data: *;".to_string(),p.lexer.position().0);
    }

    #[test]
    fn test_preprocess() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/import-smoke.dp").expect("cannot load file");
        let mut p = Parser::new(lexer);
        assert_eq!(Statement::Error("Reserved keyword 'reserved' found".to_string()),p.get_preprocessed_statement());
    }
}
