use std::str::FromStr;
use std::borrow::BorrowMut;
use std::rc::Rc;

use super::charsource::{ CharSource, LocatedCharSource };
use super::fileresolver::FileResolver;
use super::opregistry::OpRegistry;
use super::getting::LexerGetting;
use super::token::Token;

pub struct FileLexer {
    resolver: Rc<FileResolver>,
    name: String,
    stream: Result<LocatedCharSource,String>,
    ops: OpRegistry,
    pending: Option<Token>,
    line: u32,
    col: u32
}

impl FileLexer {
    pub fn new(resolver: &Rc<FileResolver>, path: &str) -> FileLexer {
        let resolver = resolver.clone();
        let stream = resolver.resolve(path);
        let first = match stream {
            Ok(_) => Token::StartOfFile(path.to_string()),
            Err(ref error) => Token::Error(error.to_string())
        };
        FileLexer {
            resolver, stream: stream.map(|x| LocatedCharSource::new(x)),
            ops: OpRegistry::new(),
            name: path.to_string(),
            pending: Some(first),
            line: 0,
            col: 0
        }
    }

    pub fn position(&self) -> (&str,u32,u32) { (&self.name,self.line,self.col) }

    pub fn add_operator(&mut self, op: &str) {
        self.ops.add(op);
    }

    fn more(&mut self) -> Token {
        loop {
            let mut getting = LexerGetting::new();
            let stream = &mut self.stream;
            if let Ok(ref mut stream) = stream {
                let (line,col) = stream.position();
                self.line = line;
                self.col = col;
                getting.go(stream,&self.ops);
                if let Some(token) = getting.make_token() {
                    return token;
                }
            } else {
                return Token::EndOfFile(None);
            }
        }
    }

    pub fn peek(&mut self) -> &Token {
        if self.pending.is_none() {
            self.pending = Some(self.more());
        }
        self.pending.as_ref().unwrap()
    }

    pub fn get(&mut self) -> Token {
        if self.pending.is_some() {
            self.pending.take().unwrap()
        } else {
            self.more()
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs::read_to_string;

    use super::super::token::Token;
    use super::super::charsource::StringCharSource;
    use super::*;
    use crate::testsuite::load_testdata;

    fn add_token(out: &mut String, token: &(Token,String,u32,u32)) {
        out.push_str(&format!("{:?} {} {},{}\n",token.0,token.1,token.2,token.3));
    }

    fn try_lex(path_in: &str) -> Vec<(Token,String,u32,u32)> {
        let mut path = String::from_str("test:").ok().unwrap();
        path.push_str(path_in);
        let resolver = Rc::new(FileResolver::new());
        let mut lexer = FileLexer::new(&resolver,&path);
        lexer.add_operator(":=");
        lexer.add_operator("==");
        lexer.add_operator("+");
        lexer.add_operator("-");
        let mut out = Vec::new();
        loop {
            let lx = &mut lexer;
            let tok = lx.get();
            if let Token::EndOfFile(_) = tok { break; }
            let (name,line,col) = lx.position();
            out.push((tok.clone(),name.to_string(),line,col));
        }
        out
    }

    fn compare_result(result: &Vec<(Token,String,u32,u32)>, path: &[&str]) {
        let outdata = load_testdata(path).ok().unwrap();
        let mut res_str = String::new();
        for r in result {
            add_token(&mut res_str,r);
        }
        if res_str != outdata {
            assert_eq!(&res_str,&outdata,"Output does not match\nEXPECTED:\n{}\nACTUAL:\n{}\n",outdata,res_str);
        }
    }

    #[test]
    fn lexer_smoke() {
        let res = try_lex("lexer/smoke.in");
        compare_result(&res,&["lexer","smoke.out"]);
    }

    #[test]
    fn lexer_operator() {
        let res = try_lex("lexer/operator.in");
        compare_result(&res,&["lexer","operator.out"]);
    }

    #[test]
    fn missing() {
        let res = try_lex("missing");
        compare_result(&res,&["lexer","missing.out"]);
    }
}