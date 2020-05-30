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

use std::collections::HashSet;
use super::charsource::{ CharSource, LocatedCharSource };
use super::inlinetokens::InlineTokens;
use super::getting::LexerGetting;
use super::token::Token;
use crate::resolver::Resolver;

pub struct FileLexer {
    resolver: Resolver,
    stream: LocatedCharSource,
    shorts: HashSet<String>,
    module: String,
    line: u32,
    col: u32
}

impl FileLexer {
    pub fn new(resolver: Resolver, stream: Box<dyn CharSource>) -> FileLexer {
        let module = stream.module().to_string();
        let mut out = FileLexer {
            stream: LocatedCharSource::new(stream),
            shorts: HashSet::new(),
            module, resolver,
            line: 0,
            col: 0
        };
        out.shorts.insert("preamble".to_string());
        out
    }

    pub fn get_module(&self) -> &str { &self.module }
    pub fn set_module(&mut self, module: &str) { self.module = module.to_string(); }
    pub fn get_resolver(&self) -> &Resolver { &self.resolver }

    pub fn add_short(&mut self, name: &str) { self.shorts.insert(name.to_string()); }
    pub fn get_shorts(&self) -> &HashSet<String> { &self.shorts }

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

    pub fn peek_multi(&mut self, ops: &InlineTokens, mode: Option<bool>, num: usize) -> Vec<Token> {
        let pos = self.stream.pos();
        let tokens = (0..num).map(|_| self.get(ops,mode)).collect::<Vec<_>>();
        self.stream.retreat(self.stream.pos()-pos);
        tokens
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
    use crate::resolver::test_resolver;

    fn add_token(out: &mut String, token: &(Token,String,u32,u32)) {
        out.push_str(&format!("{:?} {} {},{}\n",token.0,token.1,token.2,token.3));
    }

    fn try_lex(path_in: &str) -> Vec<(Token,String,u32,u32)> {
        let mut path = String::from_str("search:").ok().unwrap();
        path.push_str(path_in);
        let resolver = Rc::new(test_resolver().expect("a"));
        let source = resolver.resolve(&path);
        let (stream,resolver) = source.ok().unwrap();
        let mut lexer = FileLexer::new(resolver,stream);
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
        let res = try_lex("lexer/smoke");
        compare_result(&res,&["lexer","smoke.out"]);
    }

    #[test]
    fn lexer_operator() {
        let res = try_lex("lexer/operator");
        compare_result(&res,&["lexer","operator.out"]);
    }
}