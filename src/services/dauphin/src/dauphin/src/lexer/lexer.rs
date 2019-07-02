use std::str::FromStr;
use std::borrow::BorrowMut;
use std::rc::Rc;

use super::charsource::CharSource;
use super::charstream::CharStream;
use super::fileresolver::FileResolver;
use super::opregistry::OpRegistry;
use super::getting::LexerGetting;
use super::token::Token;

#[derive(Debug)]
pub struct LexerToken {
    pub token: Token,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32
}

pub struct Lexer {
    resolver: Rc<FileResolver>,
    stream: Result<CharStream,String>,
    ops: OpRegistry,
    pending: Option<LexerToken>
}

impl Lexer {
    pub fn new(resolver: &Rc<FileResolver>, path: &str) -> Lexer {
        let resolver = resolver.clone();
        let stream = resolver.resolve(path).map(|x| CharStream::new(x,path));
        let first = match stream {
            Ok(_) => LexerToken {
                    token: Token::StartOfFile(path.to_string()),
                    start_line: 1, start_col: 1, end_line: 1, end_col: 1
                },
            Err(ref error) => LexerToken {
                    token: Token::Error(error.to_string()),
                    start_line: 0, end_line: 0,
                    start_col: 0, end_col: 0
                }
        };
        Lexer {
            resolver, stream,
            ops: OpRegistry::new(),
            pending: Some(first)
        }
    }

    pub fn add_operator(&mut self, op: &str) {
        self.ops.add(op);
    }

    fn more(&mut self) -> LexerToken {
        loop {
            let mut getting = LexerGetting::new();
            let stream = &mut self.stream;
            if let Ok(ref mut stream) = stream {
                let (start_line,start_col) = stream.position();
                getting.go(stream,&self.ops);
                if let Some(token) = getting.make_token() {
                    let (end_line,end_col) = stream.position();
                    return LexerToken {
                        token, start_line, start_col, end_line, end_col
                    };
                }
            } else {
                return LexerToken {
                    token: Token::EndOfFile(None),
                    start_line: 0, end_line: 0,
                    start_col: 0, end_col: 0
                }
            }
        }
    }

    pub fn peek(&mut self) -> &LexerToken {
        if self.pending.is_none() {
            self.pending = Some(self.more());
        }
        self.pending.as_ref().unwrap()
    }

    pub fn get(&mut self) -> LexerToken {
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

    fn add_token(out: &mut String, token: &LexerToken) {
        out.push_str(&format!("{:?} {},{} {},{}\n",token.token,token.start_line,token.start_col,token.end_line,token.end_col));
    }

    fn try_lex(path_in: &str) -> Vec<LexerToken> {
        let mut path = String::from_str("test:").ok().unwrap();
        path.push_str(path_in);
        let resolver = Rc::new(FileResolver::new());
        let mut lexer = Lexer::new(&resolver,&path);
        lexer.add_operator(":=");
        lexer.add_operator("==");
        lexer.add_operator("+");
        lexer.add_operator("-");
        let mut out = Vec::new();
        loop {
            let tok = lexer.get();
            if let Token::EndOfFile(_) = tok.token { break; }
            out.push(tok);
        }
        out
    }

    fn compare_result(result: &Vec<LexerToken>, path: &[&str]) {
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