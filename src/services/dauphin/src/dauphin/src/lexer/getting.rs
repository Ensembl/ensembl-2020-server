use super::charsource::CharSource;
use super::opregistry::OpRegistry;
use super::token::Token;

pub struct LexerGetting {
    token: Option<Token>,
    error: Option<String>
}

fn identifier_stuff(c: char) -> bool {
    c.is_alphanumeric() && !c.is_ascii_digit() || c == '_'
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

    fn advance_char<F>(&mut self, cb: F, allow_bs : bool, stream: &mut dyn CharSource) -> String where F: Fn(char) -> bool {
        let mut out = String::new();
        let mut bs = false;
        while let Some(c) = stream.peek(1).chars().next() {
            if bs {
                out.push(c);
                bs = false;
            } else if allow_bs && c == '\\' {
                bs = true;
            } else if cb(c) {
                out.push(c);
            } else {
                break;
            }
            stream.advance(1);
        }
        if bs {
            self.set_error("trailing backslash");
        }
        out
    }

    fn get_identifier(&mut self, stream: &mut dyn CharSource) {
        let out = self.advance_char(|c| identifier_stuff(c),false,stream);
        self.set_token(Token::Identifier(out));
    }

    fn get_number(&mut self, stream: &mut dyn CharSource) {
        let out = self.advance_char(|c| c.is_ascii_digit() || c == '.',false,stream);
        if let Some(num) = out.parse::<f64>().ok() {
            self.set_token(Token::Number(num));
        } else if let Some(num) = out.parse::<i64>().ok() {
            self.set_token(Token::Number(num as f64));
        } else {
            self.set_error(&format!("Bad number \"{}\"",out));
        }
    }

    fn consume_comment(&mut self, stream: &mut dyn CharSource) {
        stream.advance(2);
        loop {
            self.advance_char(|c| c != '*',false,stream);
            let peek = stream.peek(2);
            if peek == "*/" || peek == "*" || peek == "" { break; }
            stream.advance(1);
        }
        if stream.advance(2) != "*/" {
            self.set_error("Unterminated comment");
        }
    }

    fn consume_string(&mut self, stream: &mut dyn CharSource) {
        stream.advance(1);
        let out = self.advance_char(|c| c != '"',true,stream);
        if stream.advance(1) != "\"" {
            self.set_error("Unterminated string literal");
        }
        self.set_token(Token::LiteralString(out));
    }

    fn consume_bytes(&mut self, stream: &mut dyn CharSource) {
        stream.advance(1);
        let out = self.advance_char(|c| c != '\'',true,stream);
        if stream.advance(1) != "'" {
            self.set_error("Unterminated bytes literal");
        }
        if let Ok(val) = hex::decode(&out) {
            self.set_token(Token::LiteralBytes(val));
        } else {
            self.set_error(&format!("Bad hex '{}'",out));
        }
    }

    pub fn go(&mut self, stream: &mut dyn CharSource, ops: &OpRegistry) {
        if let Some(c) = stream.peek(1).chars().next() {
            if identifier_stuff(c) {
                self.get_identifier(stream);
            } else if c.is_ascii_digit() {
                self.get_number(stream);
            } else if let Some(op) = ops.contains(stream) {
                self.set_token(Token::Operator(op));
            } else if c.is_whitespace() {
                self.advance_char(|c| c.is_whitespace(),false,stream);
            } else if c == '/' && stream.peek(2) == "/*" {
                self.consume_comment(stream);
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
