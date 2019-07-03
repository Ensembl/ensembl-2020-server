use crate::lexer::{ Lexer, Token };

use super::inline::{ Inline, InlineStore, InlineMode };
use super::node::{ Statement, ParseError };
use super::lexutil::{ get_string, get_other, not_reserved, get_identifier, get_number, get_operator };
use super::preproc::preprocess;

/* https://steinwaywu.ghost.io/operator-precedence-climbing-parsing-with-user-defined-operators/ */

struct Parser {
    lexer: Lexer,
    inlines: InlineStore
}

impl Parser {
    fn new(lexer: Lexer) -> Parser {
        Parser {
            lexer,
            inlines: InlineStore::new()
        }
    }

    fn parse_import(&mut self) -> Result<Statement,Vec<ParseError>> {
        self.lexer.get();
        Ok(Statement::Import(get_string(&mut self.lexer)?))
    }

    fn parse_inline(&mut self) -> Result<Statement,Vec<ParseError>> {
        self.lexer.get();
        let symbol = get_string(&mut self.lexer)?;
        let name = get_identifier(&mut self.lexer)?;
        let mode = match &get_identifier(&mut self.lexer)?[..] {
            "left" => Ok(InlineMode::LeftAssoc),
            "right" => Ok(InlineMode::RightAssoc),
            "prefix" => Ok(InlineMode::Prefix),
            x => Err(vec![ParseError::new("Bad oper mode, expected left, right, or prefix",&mut self.lexer)])
        }?;
        let prio = get_number(&mut self.lexer)?;
        Ok(Statement::Inline(symbol,name,mode,prio))
    }

    fn parse_regular(&mut self) -> Result<Statement,Vec<ParseError>> {
        // TODO
        get_identifier(&mut self.lexer)?;
        let op = get_operator(&mut self.lexer)?;
        get_identifier(&mut self.lexer)?;
        Ok(Statement::Regular(op))
    }

    fn parse_statement(&mut self) -> Result<Statement,Vec<ParseError>> {
        let token = self.lexer.peek();
        if let Token::Identifier(id) = token {
            let out = match &id[..] {
                "import" => self.parse_import(),
                "inline" => self.parse_inline(),
                x => {
                    not_reserved(&x.to_string(),&mut self.lexer)?;
                    self.parse_regular()
                }
            }?;
            get_other(&mut self.lexer,";")?;
            Ok(out)
        } else {
            Err(vec![ParseError::new("TODO",&mut self.lexer)])
        }
    }

    fn preprocess_stmt(&mut self, stmt: Statement) -> Result<Option<Statement>,Vec<ParseError>> {
        preprocess(&stmt,&mut self.lexer,&mut self.inlines).map(|done| if done { None } else { Some(stmt) })
    }

    fn try_get_preprocessed_statement(&mut self) -> Result<Option<Statement>,Vec<ParseError>> {
        self.parse_statement().and_then(|stmt| self.preprocess_stmt(stmt))
    }

    fn get_preprocessed_statement(&mut self) -> Result<Statement,Vec<ParseError>> {
        loop {
            match self.try_get_preprocessed_statement() {
                Ok(Some(stmt)) => return Ok(stmt),
                Ok(None) => (),
                Err(errors) => return Err(errors)
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
        assert_eq!(Ok(Statement::Import("x".to_string())),p.parse_statement());
    }

    #[test]
    fn import_statement() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("data: import \"data: *;\";").ok();
        let mut p = Parser::new(lexer);
        p.parse_statement().map(|stmt| preprocess(&stmt,&mut p.lexer,&mut p.inlines)).expect("failed");
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
        assert_eq!("Reserved keyword 'reserved' found at line 1 column 1 in test:parser/import-smoke2.dp",p.get_preprocessed_statement().err().unwrap()[0].message());
    }

    #[test]
    fn test_inline() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/inline-smoke.dp").expect("cannot load file");
        let mut p = Parser::new(lexer);
        assert_eq!(Ok(Statement::Regular(":=".to_string())),p.get_preprocessed_statement());
    }
}
