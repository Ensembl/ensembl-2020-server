use crate::lexer::{ Lexer, Token };
use crate::codegen::{ DefStore, InlineMode };

use super::node::{ Statement, ParserStatement, ParseError, Expression };
use super::lexutil::{ get_string, get_other, not_reserved, get_identifier, get_number, get_operator };
use super::preproc::preprocess;

struct Parser {
    lexer: Lexer,
    defstore: DefStore,
    stmts: Vec<Statement>,
    errors: Vec<ParseError>
}

impl Parser {
    fn new(lexer: Lexer) -> Parser {
        let mut p = Parser {
            lexer,
            defstore: DefStore::new(),
            stmts: Vec::new(),
            errors: Vec::new()
        };
        p.lexer.import("preamble:").ok();
        p
    }

    fn parse_import(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        Ok(ParserStatement::Import(get_string(&mut self.lexer)?))
    }

    fn parse_inline(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let symbol = get_string(&mut self.lexer)?;
        let name = get_identifier(&mut self.lexer)?;
        let mode = match &get_identifier(&mut self.lexer)?[..] {
            "left" => Ok(InlineMode::LeftAssoc),
            "right" => Ok(InlineMode::RightAssoc),
            "prefix" => Ok(InlineMode::Prefix),
            "suffix" => Ok(InlineMode::Suffix),
            _ => Err(ParseError::new("Bad oper mode, expected left, right, prefix, or suffix",&mut self.lexer))
        }?;
        let prio = get_number(&mut self.lexer)?;
        Ok(ParserStatement::Inline(symbol,name,mode,prio))
    }

    fn parse_exprdecl(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let name = get_identifier(&mut self.lexer)?;
        Ok(ParserStatement::ExprMacro(name.to_string()))
    }

    fn parse_stmtdecl(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let name = get_identifier(&mut self.lexer)?;
        Ok(ParserStatement::StmtMacro(name.to_string()))
    }

    fn parse_func(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let name = get_identifier(&mut self.lexer)?;
        Ok(ParserStatement::FuncDecl(name.to_string()))
    }

    fn parse_proc(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let name = get_identifier(&mut self.lexer)?;
        Ok(ParserStatement::ProcDecl(name.to_string()))
    }

    fn parse_struct(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let name = get_identifier(&mut self.lexer)?;
        Ok(ParserStatement::StructDef(name.to_string()))
    }

    fn parse_enum(&mut self) -> Result<ParserStatement,ParseError> {
        self.lexer.get();
        let name = get_identifier(&mut self.lexer)?;
        Ok(ParserStatement::EnumDef(name.to_string()))
    }

    fn parse_atom_id(&mut self,id: &str, nested: bool) -> Result<Expression,ParseError> {
        if self.defstore.stmt_like(id,&mut self.lexer).unwrap_or(false) {
            Err(ParseError::new("Unexpected statement in expression",&mut self.lexer))?;
        }
        if !self.defstore.stmt_like(id, &mut self.lexer).unwrap_or(true) { /* expr-like */
            get_other(&mut self.lexer, "(")?;
            Ok(Expression::Operator(id.to_string(),self.parse_exprlist(')',nested)?))
        } else {
            Ok(match id {
                "true" => Expression::LiteralBool(true),
                "false" => Expression::LiteralBool(true),
                id => Expression::Identifier(id.to_string())
            })
        }
    }

    fn require_filter(&mut self, c: char, nested: bool) -> Result<(),ParseError> {
        if !nested {
            return Err(ParseError::new(&format!("{} encountered outside filter",c),&mut self.lexer));
        }
        Ok(())
    }

    fn parse_atom(&mut self, nested: bool) -> Result<Expression,ParseError> {
        Ok(match self.lexer.get() {
            Token::Identifier(id) => self.parse_atom_id(&id,nested)?,
            Token::Number(num,_) => Expression::Number(num),
            Token::LiteralString(s) => Expression::LiteralString(s),
            Token::LiteralBytes(b) => Expression::LiteralBytes(b),
            Token::Other('(') => {
                let out = self.parse_expr(nested)?;
                get_other(&mut self.lexer,")")?;
                out
            },
            Token::Other('$') => {
                self.require_filter('$',nested)?;
                Expression::Dollar
            },
            Token::Other('@') => {
                self.require_filter('@',nested)?;
                Expression::At
            },
            Token::Operator(op) => self.parse_prefix(&op,nested)?,
            _ => Err(ParseError::new(&format!("Expected expression"),&mut self.lexer))?
        })
    }

    fn vec_ctor(&mut self, nested: bool) -> Result<Expression,ParseError> {
        Ok(Expression::Vector(self.parse_exprlist(']',nested)?))
    }

    fn parse_prefix(&mut self, op: &str, nested: bool) -> Result<Expression,ParseError> {
        if self.defstore.stmt_like(op, &mut self.lexer).unwrap_or(false) { /* stmt-like */
            return Err(ParseError::new("Unexpected statement",&mut self.lexer));
        }
        let inline = self.defstore.get_inline_unary(op,&mut self.lexer)?;
        let prec = inline.precedence();
        if inline.mode() != &InlineMode::Prefix {
            return Err(ParseError::new("Not a prefix operator",&mut self.lexer));
        }
        let name = inline.name().to_string();
        Ok(match &name[..] {
            "__star__" => Expression::Star(Box::new(self.parse_expr_level(Some(prec),true,nested)?)),
            "__sqctor__" => self.vec_ctor(nested)?,
            _ => Expression::Operator(name.to_string(),vec![self.parse_expr_level(Some(prec),true,nested)?])
        })
    }

    fn parse_binary_right(&mut self, left: Expression, name: &str, min: f64, oreq: bool, nested: bool) -> Result<Expression,ParseError> {
        self.lexer.get();
        let right = self.parse_expr_level(Some(min),oreq,nested)?;
        Ok(Expression::Operator(name.to_string(),vec![left,right]))
    }

    fn parse_brackets(&mut self, left: Expression) -> Result<Expression,ParseError> {
        if let Token::Other(']') = self.lexer.peek() {
            self.lexer.get();
            Ok(Expression::Square(Box::new(left)))
        } else {
            let inside = self.parse_expr(true)?;
            get_other(&mut self.lexer, "]")?;
            Ok(Expression::Bracket(Box::new(left),Box::new(inside)))
        }
    }

    fn parse_suffix(&mut self, left: Expression, name: &str) -> Result<Expression,ParseError> {
        self.lexer.get();
        Ok(match &name[..] {
            "__sqopen__" => self.parse_brackets(left)?,
            "__dot__" => Expression::Dot(Box::new(left),get_identifier(&mut self.lexer)?),
            "__query__" => Expression::Query(Box::new(left),get_identifier(&mut self.lexer)?),
            "__pling__" => Expression::Pling(Box::new(left),get_identifier(&mut self.lexer)?),
            "__ref__" => {
                if get_operator(&mut self.lexer)? != "[" {
                    return Err(ParseError::new("Expected [",&self.lexer));
                }
                if let Expression::Bracket(op,key) = self.parse_brackets(left)? {
                    return Ok(Expression::Filter(op,key));
                } else {
                    return Err(ParseError::new("Expected filter",&self.lexer));
                }
            },
            _ => Expression::Operator(name.to_string(),vec![left])
        })
    }

    fn extend_expr(&mut self, left: Expression, symbol: &str, min: Option<f64>, oreq: bool, nested: bool) -> Result<(Expression,bool),ParseError> {
        let inline = self.defstore.get_inline_binary(symbol,&mut self.lexer)?;
        let prio = inline.precedence();
        if let Some(min) = min {
            if prio > min || (prio==min && !oreq) {
                return Ok((left,false));
            }
        }
        let name = inline.name().to_string();
        if self.defstore.stmt_like(&name,&mut self.lexer)? {
            return Ok((left,false));
        }
        Ok(match *inline.mode() {
            InlineMode::LeftAssoc => (self.parse_binary_right(left,&name,prio,false,nested)?,true),
            InlineMode::RightAssoc => (self.parse_binary_right(left,&name,prio,true,nested)?,true),
            InlineMode::Prefix => (left,false),
            InlineMode::Suffix => (self.parse_suffix(left,&name)?,true)
        })
    }

    fn parse_expr_level(&mut self, min: Option<f64>, oreq: bool, nested: bool) -> Result<Expression,ParseError> {
        let mut out = self.parse_atom(nested)?;
        loop {
            match self.lexer.peek() {
                Token::Operator(op) => {
                    let op = op.to_string();
                    let (expr,progress) = self.extend_expr(out,&op,min,oreq,nested)?;
                    out = expr;
                    if !progress {
                        return Ok(out);
                    }
                },
                _ => return Ok(out)
            }
        }
    }

    fn parse_expr(&mut self, nested: bool) -> Result<Expression,ParseError> {
        self.parse_expr_level(None,true,nested)
    }

    fn parse_exprlist(&mut self, term: char, nested: bool) -> Result<Vec<Expression>,ParseError> {
        let mut out = Vec::new();
        loop {
            match self.lexer.peek() {
                Token::Other(x) if x == &term => {
                    self.lexer.get();
                    return Ok(out)
                },
                Token::Other(',') => {
                    self.lexer.get();
                },
                _ => {
                    out.push(self.parse_expr(nested)?);
                }
            }
        }
    }

    fn parse_funcstmt(&mut self)-> Result<ParserStatement,ParseError> {
        let name = get_identifier(&mut self.lexer)?;
        get_other(&mut self.lexer,"(")?;
        let exprs = self.parse_exprlist(')',false)?;
        Ok(ParserStatement::Regular(Statement(name,exprs)))
    } 

    fn parse_inlinestmt(&mut self)-> Result<ParserStatement,ParseError> {
        let left = self.parse_expr(false)?;
        let op = get_operator(&mut self.lexer)?;
        let right = self.parse_expr(false)?;
        let name = self.defstore.get_inline_binary(&op,&mut self.lexer)?.name();
        if !self.defstore.stmt_like(&name,&mut self.lexer)? {
            Err(ParseError::new("Got inline expr, expected inline stmt",&mut self.lexer))?;
        }
        Ok(ParserStatement::Regular(Statement(name.to_string(),vec![left,right])))
    }

    fn parse_regular(&mut self) -> Result<ParserStatement,ParseError> {
        let lexer = &mut self.lexer;
        if let Token::Identifier(id) = lexer.peek() {
            let id = id.clone();
            if self.defstore.stmt_like(&id,lexer).unwrap_or(false) {
                return self.parse_funcstmt();
            }
        }
        self.parse_inlinestmt()
    }

    fn try_parse_statement(&mut self) -> Result<Option<ParserStatement>,ParseError> {
        let token = self.lexer.peek();
        match token {
            Token::Identifier(id) => {
                let out = match &id[..] {
                    "import" => self.parse_import(),
                    "inline" => self.parse_inline(),
                    "expr" => self.parse_exprdecl(),
                    "stmt" => self.parse_stmtdecl(),
                    "func" => self.parse_func(),
                    "proc" => self.parse_proc(),
                    "struct" => self.parse_struct(),
                    "enum" => self.parse_enum(),
                    x => {
                        not_reserved(&x.to_string(),&mut self.lexer)?;
                        self.parse_regular()
                    }
                }?;
                get_other(&mut self.lexer,";")?;
                Ok(Some(out))
            },
            Token::EndOfFile => { self.lexer.get(); Ok(None) },
            Token::EndOfLex => Ok(Some(ParserStatement::EndOfParse)),
            _ => {
                let out = self.parse_regular()?;
                get_other(&mut self.lexer,";")?;
                Ok(Some(out))
            }            
        }
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

    fn parse_statement(&mut self) -> Result<ParserStatement,ParseError> {
        loop {
            let s = self.try_parse_statement();
            if s.is_err() {
                self.ffwd_error();
                return Err(s.err().unwrap());
            }
            if let Ok(Some(stmt)) = s {
                return Ok(stmt)
            }
        }
    }

    fn preprocess_stmt(&mut self, stmt: ParserStatement) -> Result<Option<ParserStatement>,ParseError> {
        preprocess(&stmt,&mut self.lexer,&mut self.defstore).map(|done| if done { None } else { Some(stmt) })
    }

    fn try_get_preprocessed_statement(&mut self) -> Result<Option<ParserStatement>,ParseError> {
        self.parse_statement().and_then(|stmt| self.preprocess_stmt(stmt))
    }

    fn parse(mut self) -> Result<(Vec<Statement>,DefStore),Vec<ParseError>> {
        loop {
            match self.try_get_preprocessed_statement() {
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
    use super::super::preproc::preprocess;
    use crate::lexer::FileResolver;
    use crate::testsuite::load_testdata;

    fn last_statement(p: &mut Parser) -> Result<ParserStatement,ParseError> {
        let mut prev = Err(ParseError::new("unexpected EOF",&mut p.lexer));
        loop {
            match p.parse_statement()? {
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
        let mut p = Parser::new(lexer);
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
        let txt = "\'assign\' already defined at test:parser/id-clash.dp 1:12 at line 2 column 14 in test:parser/id-clash.dp";
        assert_eq!(txt,p.parse().err().unwrap()[0].message());
    }
}
