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

use super::filelexer::FileLexer;
use super::fileresolver::FileResolver;
use super::inlinetokens::InlineTokens;
use super::token::Token;

pub struct Lexer {
    resolver: FileResolver,
    files: Vec<FileLexer>,
    inlines: InlineTokens
}

impl Lexer {
    pub fn new(resolver: FileResolver) -> Lexer {
        Lexer {
            resolver,
            inlines: InlineTokens::new(),
            files: Vec::new()
        }
    }

    pub fn add_inline(&mut self, s: &str, mode: bool) -> Result<(),String> {
        self.inlines.add(s,mode)
    }

    pub fn import(&mut self, path: &str) -> Result<(),String> {
        self.resolver.resolve(path).map(|stream| {
            self.files.push(FileLexer::new(stream)); ()
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

    #[test]
    fn smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:lexer/smoke2.in").expect("import failed");
        let mut out = String::new();
        loop {
            let lx = &mut lexer;
            let tok = lx.get().clone();
            let (name,line,col) = lx.position();
            let name = name.to_string();
            if let Token::EndOfLex = tok { break; }
            if let Token::Identifier(ref s) = tok {
                if s == "import" {
                    lx.import("test:lexer/smoke2b.in").expect("import failed");
                }
            }
            out.push_str(&format!("{:?} {} {},{}\n",tok,name,line,col));
        }
        let outdata = load_testdata(&["lexer","smoke2.out"]).ok().unwrap();
        assert_eq!(out,outdata,"EXPECTED:\n{}\nACTUAL:\n{}\n",outdata,out);
    }

    #[test]
    fn missing() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        assert_eq!(lexer.import("test:missing").err().unwrap(),"Loading \"missing\": No such file or directory (os error 2)","Error message missing");
    }
}
