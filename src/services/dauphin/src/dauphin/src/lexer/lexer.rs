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
use super::filelexer::FileLexer;
use crate::resolver::Resolver;
use super::inlinetokens::InlineTokens;
use super::token::Token;

pub struct Lexer<'a> {
    empty_set: HashSet<String>,
    resolver: &'a Resolver,
    files: Vec<FileLexer>,
    inlines: InlineTokens
}

impl<'a> Lexer<'a> {
    pub fn new(resolver: &'a Resolver) -> Lexer<'a> {
        Lexer {
            empty_set: HashSet::new(),
            resolver,
            inlines: InlineTokens::new(),
            files: Vec::new()
        }
    }

    pub fn get_module(&self) -> &str {
        self.files.last().map(|f| f.get_module()).unwrap_or("")
    }

    pub fn set_module(&mut self, module: &str) { 
        if let Some(last) = self.files.last_mut() {
            last.set_module(module);
        }
    }

    pub fn add_short(&mut self, name: &str) {
        if let Some(last) = self.files.last_mut() {
            last.add_short(name);
        }
    }

    pub fn get_shorts(&self) -> &HashSet<String> {
        if let Some(last) = self.files.last() {
            last.get_shorts()
        } else {
            &self.empty_set
        }
    }

    pub fn add_inline(&mut self, s: &str, mode: bool) -> Result<(),String> {
        self.inlines.add(s,mode)
    }

    pub fn import(&mut self, path: &str) -> Result<(),String> {
        let resolver = self.files.iter().last().map(|f| f.get_resolver()).unwrap_or_else(|| &self.resolver);
        resolver.resolve(path).map(|stream| {
            self.files.push(FileLexer::new(stream.1,stream.0)); ()
        })
    }

    pub fn position(&self) -> (&str,u32,u32) {
        if let Some(last) = self.files.last() {
            last.position()
        } else {
            ("EOF",0,0)
        }
    }

    pub fn peek(&mut self, mode: Option<bool>, num: usize) -> Vec<Token> {
        if let Some(last) = self.files.last_mut() {
            last.peek_multi(&self.inlines,mode,num)
        } else {
            vec![Token::EndOfLex]
        }
    }

    fn more(&mut self, allow_ops: Option<bool>) -> Token {
        if let Some(last) = self.files.last_mut() {
            let tok = last.get(&self.inlines,allow_ops);
            if let Token::EndOfFile = tok {
                self.files.pop();
            }
            tok
        } else {
            Token::EndOfLex
        }
    }

    pub fn get(&mut self) -> Token {
        self.more(None)
    }

    pub fn get_oper(&mut self, mode: bool) -> Token {
        self.more(Some(mode))
    }

    pub fn pos(&self) -> usize {
        if let Some(last) = self.files.last() {
            last.pos()
        } else {
            0
        }
    }

    pub fn back_to(&mut self, pos: usize) {
        if let Some(last) = self.files.last_mut() {
            last.back_to(pos);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::files::load_testdata;
    use crate::resolver::test_resolver;

    #[test]
    fn lexer_smoke() {
        let resolver = test_resolver().expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:lexer/smoke2").expect("import failed");
        let mut out = String::new();
        loop {
            let lx = &mut lexer;
            let tok = lx.get().clone();
            let (name,line,col) = lx.position();
            let name = name.to_string();
            if let Token::EndOfLex = tok { break; }
            if let Token::Identifier(ref s) = tok {
                if s == "import" {
                    lx.import("search:lexer/smoke2b").expect("import failed");
                }
            }
            out.push_str(&format!("{:?} {} {},{}\n",tok,name,line,col));
        }
        let outdata = load_testdata(&["lexer","smoke2.out"]).ok().unwrap();
        assert_eq!(out,outdata,"EXPECTED:\n{}\nACTUAL:\n{}\n",outdata,out);
    }

    #[test]
    fn missing() {
        let resolver = test_resolver().expect("a");
        let mut lexer = Lexer::new(&resolver);
        assert!(lexer.import("file:missing").err().unwrap().contains("No such file or directory"));
    }
}
