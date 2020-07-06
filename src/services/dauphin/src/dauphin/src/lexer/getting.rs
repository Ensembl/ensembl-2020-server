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

use super::charsource::CharSource;
use super::inlinetokens::InlineTokens;
use super::token::Token;

pub struct LexerGetting {
    token: Option<Token>,
    error: Option<String>
}

fn identifier_stuff(c: char,initial: bool) -> bool {
    c.is_alphanumeric() && (!c.is_ascii_digit() || !initial) || c == '_'
}

impl LexerGetting {
    pub fn new() -> LexerGetting {
        LexerGetting { token: None, error: None }
    }

    fn set_token(&mut self, tok: Token) {
        self.token = Some(tok);
    }

    fn set_error(&mut self, error: &str) {
        if self.error.is_none() {
            self.error = Some(error.to_string());
        }
    }

    pub fn make_token(&mut self) -> Option<Token> {
        if self.error.is_some() {
            Some(Token::Error(self.error.take().unwrap()))
        } else if self.token.is_some() {
            Some(self.token.take().unwrap())
        } else {
            None
        }
    }

    fn advance_char<F>(&mut self, cb: F, allow_bs : bool, stream: &mut dyn CharSource) -> String where F: Fn(char,bool) -> bool {
        let mut out = String::new();
        let mut bs = false;
        let mut first = true;
        while let Some(c) = stream.peek(1).chars().next() {
            if bs {
                out.push(c);
                bs = false;
            } else if allow_bs && c == '\\' {
                bs = true;
            } else if cb(c,first) {
                out.push(c);
            } else {
                break;
            }
            first = false;
            stream.advance(1);
        }
        if bs {
            self.set_error("trailing backslash");
        }
        out
    }

    fn get_identifier(&mut self, stream: &mut dyn CharSource) {
        let out = self.advance_char(|c,first| identifier_stuff(c,first),false,stream);
        self.set_token(Token::Identifier(out));
    }

    fn get_number(&mut self, stream: &mut dyn CharSource) {
        let out = self.advance_char(|c,_| c.is_ascii_digit() || c == '.',false,stream);
        self.set_token(Token::Number(out));
    }

    fn consume_comment(&mut self, stream: &mut dyn CharSource) {
        stream.advance(2);
        loop {
            self.advance_char(|c,_| c != '*',false,stream);
            let peek = stream.peek(2);
            if peek == "*/" || peek == "*" || peek == "" { break; }
            stream.advance(1);
        }
        if stream.advance(2) != "*/" {
            self.set_error("Unterminated comment");
        }
    }

    fn consume_to_eol(&mut self, stream: &mut dyn CharSource) {
        self.advance_char(|c,_| c != '\n',false,stream);
    }

    fn consume_string(&mut self, stream: &mut dyn CharSource) {
        stream.advance(1);
        let out = self.advance_char(|c,_| c != '"',true,stream);
        if stream.advance(1) != "\"" {
            self.set_error("Unterminated string literal");
        }
        self.set_token(Token::LiteralString(out));
    }

    fn consume_bytes(&mut self, stream: &mut dyn CharSource) {
        stream.advance(1);
        let out = self.advance_char(|c,_| c != '\'',true,stream);
        if stream.advance(1) != "'" {
            self.set_error("Unterminated bytes literal");
        }
        if let Ok(val) = hex::decode(&out) {
            self.set_token(Token::LiteralBytes(val));
        } else {
            self.set_error(&format!("Bad hex '{}'",out));
        }
    }

    fn ops_test(&self, stream: &mut dyn CharSource, ops: &InlineTokens, mode: Option<bool>) -> Option<String> {
        if let Some(mode) = mode {
            ops.contains(stream,mode)
        } else {
            None
        }
    }

    pub fn go(&mut self, stream: &mut dyn CharSource, ops: &InlineTokens, mode: Option<bool>) {
        if let Some(c) = stream.peek(1).chars().next() {
            if identifier_stuff(c,true) {
                self.get_identifier(stream);
            } else if c.is_ascii_digit() {
                self.get_number(stream);
            } else if let Some(op) = self.ops_test(stream,ops,mode) {
                self.set_token(Token::Operator(op));
            } else if c.is_whitespace() {
                self.advance_char(|c,_| c.is_whitespace(),false,stream);
            } else if c == ':' && stream.peek(2) == "::" {
                stream.advance(2);
                self.set_token(Token::FourDots);
            } else if c == '/' && stream.peek(2) == "/*" {
                self.consume_comment(stream);
            } else if c == '/' && stream.peek(2) == "//" {
                self.consume_to_eol(stream);
            } else if c == '"' {
                self.consume_string(stream);
            } else if c == '\'' {
                self.consume_bytes(stream);
            } else {
                stream.advance(1);
                self.set_token(Token::Other(c));
            }
        } else {
            self.set_token(Token::EndOfFile);
        }
    }
}
