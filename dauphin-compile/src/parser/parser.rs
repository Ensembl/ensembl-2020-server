/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use crate::lexer::{ Lexer, Token };
use crate::model::DefStore;

use super::node::{ Statement, ParserStatement, ParseError };
use super::declare::declare;
use super::parsestmt::{ parse_statement };

pub struct Parser<'a> {
    lexer: &'a mut Lexer<'a>,
    defstore: DefStore,
    stmts: Vec<Statement>,
    errors: Vec<ParseError>
}

impl<'a> Parser<'a> {
    pub fn new(lexer: &'a mut Lexer<'a>) -> Parser<'a> {
        let source = lexer.get_source().to_string();
        let p = Parser {
            lexer,
            defstore: DefStore::new(&source),
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

    fn recover_parse_statement(&mut self) -> Result<Vec<ParserStatement>,ParseError> {
        let s = parse_statement(&mut self.lexer,&self.defstore,false);
        if s.is_err() {
            self.ffwd_error();
            return Err(s.err().unwrap());
        }
        s
    }

    fn run_declare(&mut self, stmt: &ParserStatement) -> Result<bool,ParseError> {
        declare(&stmt,&mut self.lexer,&mut self.defstore)
    }

    fn get_non_declare(&mut self) -> Result<Vec<ParserStatement>,ParseError> {
        let mut out = vec![];
        for stmt in &self.recover_parse_statement()? {
            if !self.run_declare(stmt)? {
                out.push(stmt.clone());
            }
        }
        Ok(out)
    }

    pub fn parse(mut self) -> Result<(Vec<Statement>,DefStore),Vec<ParseError>> {
        loop {
            match self.get_non_declare() {
                Ok(mut stmts) => {
                    for stmt in stmts.drain(..) {
                        match stmt {
                            ParserStatement::EndOfParse => {
                                if self.errors.len() > 0 {
                                    return Err(self.errors)
                                } else {
                                    return Ok((self.stmts,self.defstore))
                                }
                            },
                            ParserStatement::Regular(stmt) =>  self.stmts.push(stmt),
                            _ => (),
                        }
                    }
                },
                Err(error) => self.errors.push(error)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::resolver::common_resolver;
    use dauphin_interp::command::Identifier;
    use crate::test::{ xxx_test_config, make_compiler_suite, load_testdata };
    use crate::command::CompilerLink;

    fn last_statement(p: &mut Parser) -> Result<ParserStatement,ParseError> {
        let mut prev = Err(ParseError::new("unexpected EOF",&mut p.lexer));
        loop {
            let mut stmts = p.recover_parse_statement()?;
            match stmts.pop() {
                Some(ParserStatement::EndOfParse) => break,
                Some(x) => prev = Ok(x),
                None => ()
            }
        }
        return prev;
    }

    #[test]
    fn statement() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("data: import \"x\";").ok();
        let mut p = Parser::new(&mut lexer);

        assert_eq!(Ok(ParserStatement::Import("x".to_string())),last_statement(&mut p));
    }

    #[test]
    fn import_statement() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("data: import \"data: $;\";").ok();
        let p = Parser::new(&mut lexer);
        let err = p.parse().err().unwrap();
        assert_eq!("$ encountered outside filter at data: $;:1:2".to_string(),err[0].message());
    }

    #[test]
    fn import_search_statement() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("A");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:import-search").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let txt = "Reserved keyword 'reserved' found at file:../import-smoke4.dp:1:1";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    #[test]
    fn test_preprocess() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/import-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let txt = "Reserved keyword 'reserved' found at file:../import-smoke4.dp:1:1";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    #[test]
    fn test_smoke() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/parser-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,_defstore) = p.parse().expect("error");
        let mut out : Vec<String> = stmts.iter().map(|x| format!("{:?}",x)).collect();
        out.push("".to_string()); /* For trailing \n */
        let outdata = load_testdata(&["parser","parser-smoke.out"]).ok().unwrap();
        assert_eq!(outdata,out.join("\n"));
    }

    #[test]
    fn test_no_nested_dollar() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/parser-nonest").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let txt = "$ encountered outside filter at search:parser/parser-nonest:5:1";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    #[test]
    fn test_id_clash() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/id-clash").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let txt = "duplicate identifier: id_clash::assign at search:parser/id-clash:2:29";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }

    fn make_identifier(module: &str,name: &str) -> Identifier {
        Identifier::new(module,name)
    }

    fn print_struct(defstore: &DefStore, name: &str) -> String {
        format!("{:?}",defstore.get_struct_id(&make_identifier("struct_smoke",name)).expect("A"))
    }

    #[test]
    fn test_struct() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/struct-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        assert_eq!("struct struct_smoke::A { 0: number, 1: vec(number) }",print_struct(&defstore,"A"));
        assert_eq!("struct struct_smoke::B { X: number, Y: vec(struct_smoke::A) }",print_struct(&defstore,"B"));
        assert_eq!("struct struct_smoke::C {  }",print_struct(&defstore,"C"));
        assert_eq!("[assign(x,A {0: [1,2,3]}), assign(y,B {X: 23,Y: [x,x]})]",&format!("{:?}",stmts));
    }

    fn print_enum(defstore: &DefStore, name: &str) -> String {
        format!("{:?}",defstore.get_enum_id(&make_identifier("enum_smoke",name)).expect("A"))
    }

    #[test]
    fn test_enum() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/enum-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        assert_eq!("enum enum_smoke::A { M: number, N: vec(number) }",print_enum(&defstore,"A"));
        assert_eq!("enum enum_smoke::B { X: number, Y: vec(enum_smoke::A) }",print_enum(&defstore,"B"));
        assert_eq!("enum enum_smoke::C {  }",print_enum(&defstore,"C"));
        assert_eq!("[assign(x,B:Y [A:M 42,B:N [1,2,3]])]",&format!("{:?}",stmts));
    }

    #[test]
    fn test_short() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/short").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,_) = p.parse().expect("error");
        let modules = stmts.iter().map(|x| (x.0).module().to_string()).collect::<Vec<_>>();
        assert_eq!(vec!["library1","library2","library2",
                        "library1","library2","library1"],modules);
    }

    #[test]
    fn test_macro() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:parser/macro").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,_defstore) = p.parse().expect("error");
        let x = format!("{:?}",stmts);
        print!("{}\n",x);
        assert_eq!("[assign(x,[[1,2,3],[4,5,6],[7,8,9]]), assign(z,0), incr(((x)[eq(@,0)])[eq(@,1)],1), incr(z,plus(0,1)), assign(z,plus(z,0))]",x);
    }

}
