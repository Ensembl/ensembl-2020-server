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
 *  
 *  vscode-fold=1
 */

use super::charsource::{ CharSource, LocatedCharSource };
use super::inlinetokens::InlineTokens;
use super::getting::LexerGetting;
use super::token::Token;

pub struct FileLexer {
    stream: LocatedCharSource,
    line: u32,
    col: u32
}

impl FileLexer {
    pub fn new(stream: Box<dyn CharSource>) -> FileLexer {
        FileLexer {
            stream: LocatedCharSource::new(stream),
            line: 0,
            col: 0
        }
    }

    pub fn position(&self) -> (&str,u32,u32) { 
        (self.stream.name(),self.line,self.col)
    }

    pub fn get(&mut self, ops: &InlineTokens, mode: Option<bool>) -> Token {
        loop {
            let mut getting = LexerGetting::new();
            let stream = &mut self.stream;
            let (line,col) = stream.position();
            self.line = line;
            self.col = col;
            getting.go(stream,ops,mode);
            if let Some(token) = getting.make_token() {
                return token;
            }
        }
    }

    pub fn peek(&mut self, ops: &InlineTokens, mode: Option<bool>) -> Token {
        let pos = self.stream.pos();
        let token = self.get(ops,mode);
        self.stream.retreat(self.stream.pos()-pos);
        token
    }

    pub fn pos(&self) -> usize {
        self.stream.pos()
    }

    pub fn back_to(&mut self, pos: usize) {
        self.stream.retreat(self.stream.pos()-pos);
    }
}

#[cfg(test)]
mod test {
    use super::super::token::Token;
    use super::*;
    use crate::test::files::load_testdata;
    use std::str::FromStr;
    use std::rc::Rc;
    use super::super::fileresolver::FileResolver;

    fn add_token(out: &mut String, token: &(Token,String,u32,u32)) {
        out.push_str(&format!("{:?} {} {},{}\n",token.0,token.1,token.2,token.3));
    }

    fn try_lex(path_in: &str) -> Vec<(Token,String,u32,u32)> {
        let mut path = String::from_str("test:").ok().unwrap();
        path.push_str(path_in);
        let resolver = Rc::new(FileResolver::new());
        let source = resolver.resolve(&path);
        let mut lexer = FileLexer::new(source.ok().unwrap());
        let mut ops = InlineTokens::new();
        ops.add(":=",false).ok();
        ops.add("==",false).ok();
        ops.add("=",false).ok();
        ops.add("+",false).ok();
        ops.add("-",false).ok();
        let mut out = Vec::new();
        loop {
            let lx = &mut lexer;
            let tok = lx.get(&ops,Some(false));
            if let Token::EndOfFile = tok { break; }
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
}