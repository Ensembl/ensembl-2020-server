use crate::lexer::{ Lexer, Token };
use crate::codegen::DefStore;

use super::node::{ Statement, ParserStatement, ParseError };
use super::declare::declare;
use super::parsestmt::{ parse_statement };

pub struct Parser {
    lexer: Lexer,
    defstore: DefStore,
    stmts: Vec<Statement>,
    errors: Vec<ParseError>
}

impl Parser {
    pub fn new(lexer: Lexer) -> Parser {
        let mut p = Parser {
            lexer,
            defstore: DefStore::new(),
            stmts: Vec::new(),
            errors: Vec::new()
        };
        p.lexer.import("preamble:").ok();
        p
    }

    fn ffwd_error(&mut self) {
        loop {
            match self.lexer.get() {
                Token::Other(';') => return,
                Token::EndOfLex => return,
                _ => ()
            }
        }
    }

    fn recover_parse_statement(&mut self) -> Result<ParserStatement,ParseError> {
        loop {
            let s = parse_statement(&mut self.lexer,&self.defstore);
            if s.is_err() {
                self.ffwd_error();
                return Err(s.err().unwrap());
            }
            if let Ok(Some(stmt)) = s {
                return Ok(stmt)
            }
        }
    }

    fn run_declare(&mut self, stmt: ParserStatement) -> Result<Option<ParserStatement>,ParseError> {
        declare(&stmt,&mut self.lexer,&mut self.defstore).map(|done| if done { None } else { Some(stmt) })
    }

    fn get_non_declare(&mut self) -> Result<Option<ParserStatement>,ParseError> {
        self.recover_parse_statement().and_then(|stmt| self.run_declare(stmt))
    }

    pub fn parse(mut self) -> Result<(Vec<Statement>,DefStore),Vec<ParseError>> {
        loop {
            match self.get_non_declare() {
                Ok(Some(ParserStatement::EndOfParse)) => {
                    if self.errors.len() > 0 {
                        return Err(self.errors)
                    } else {
                        return Ok((self.stmts,self.defstore))
                    }
                },
                Ok(Some(ParserStatement::Regular(stmt))) =>  self.stmts.push(stmt),
                Err(error) => self.errors.push(error),
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::FileResolver;
    use crate::testsuite::load_testdata;

    fn last_statement(p: &mut Parser) -> Result<ParserStatement,ParseError> {
        let mut prev = Err(ParseError::new("unexpected EOF",&mut p.lexer));
        loop {
            match p.recover_parse_statement()? {
                ParserStatement::EndOfParse => break,
                x => prev = Ok(x)
            }
        }
        return prev;
    }

    #[test]
    fn statement() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("data: import \"x\";").ok();
        let mut p = Parser::new(lexer);

        assert_eq!(Ok(ParserStatement::Import("x".to_string())),last_statement(&mut p));
    }

    #[test]
    fn import_statement() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("data: import \"data: $;\";").ok();
        let p = Parser::new(lexer);
        let err = p.parse().err().unwrap();
        assert_eq!("$ encountered outside filter at line 1 column 2 in data: $;".to_string(),err[0].message());
    }

    #[test]
    fn test_preprocess() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/import-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let txt = "Reserved keyword 'reserved' found at line 1 column 1 in test:parser/import-smoke2.dp";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    #[test]
    fn test_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/parser-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,_defstore) = p.parse().expect("error");
        let mut out : Vec<String> = stmts.iter().map(|x| format!("{:?}",x)).collect();
        out.push("".to_string()); /* For trailing \n */
        let outdata = load_testdata(&["parser","parser-smoke.out"]).ok().unwrap();
        assert_eq!(outdata,out.join("\n"));
    }

    #[test]
    fn test_no_nested_dollar() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/parser-nonest.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let txt = "$ encountered outside filter at line 5 column 1 in test:parser/parser-nonest.dp";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    #[test]
    fn test_id_clash() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/id-clash.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let txt = "\'assign\' already defined at test:parser/id-clash.dp 1:12 at line 2 column 12 in test:parser/id-clash.dp";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    #[test]
    fn test_struct() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/struct-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        assert_eq!("struct A { 0: number, 1: vec(number) }",format!("{:?}",defstore.get_struct("A").unwrap()));
        assert_eq!("struct B { X: number, Y: vec(A) }",format!("{:?}",defstore.get_struct("B").unwrap()));
        assert_eq!("struct C {  }",format!("{:?}",defstore.get_struct("C").unwrap()));
        assert_eq!("[assign(x,A {0: [1,2,3]}), assign(y,B {X: 23,Y: [x,x]})]",&format!("{:?}",stmts));
    }

    #[test]
    fn test_enum() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:parser/enum-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        assert_eq!("enum A { M: number, N: vec(number) }",format!("{:?}",defstore.get_enum("A").unwrap()));
        assert_eq!("enum B { X: number, Y: vec(A) }",format!("{:?}",defstore.get_enum("B").unwrap()));
        assert_eq!("enum C {  }",format!("{:?}",defstore.get_enum("C").unwrap()));
        assert_eq!("[assign(x,B:Y [A:M 42,B:N [1,2,3]])]",&format!("{:?}",stmts));
    }
}
