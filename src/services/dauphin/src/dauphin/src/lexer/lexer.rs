use super::filelexer::FileLexer;
use super::fileresolver::FileResolver;
use super::inlinetokens::InlineTokens;
use super::token::Token;

pub struct Lexer {
    resolver: FileResolver,
    files: Vec<FileLexer>,
    inlines: InlineTokens,
    end: Token
}

impl Lexer {
    pub fn new(resolver: FileResolver) -> Lexer {
        Lexer {
            resolver,
            inlines: InlineTokens::new(),
            files: Vec::new(),
            end: Token::EndOfLex
        }
    }

    pub fn add_inline(&mut self, s: &str) {
        self.inlines.add(s);
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

    pub fn peek(&mut self) -> &Token {
        if let Some(last) = self.files.last_mut() {
            last.peek(&self.inlines)
        } else {
            &self.end
        }
    }

    pub fn get(&mut self) -> Token {
        if let Some(last) = self.files.last_mut() {
            let tok = last.get(&self.inlines);
            if let Token::EndOfFile = tok {
                self.files.pop();
            }
            tok
        } else {
            Token::EndOfLex
        }
    }

    pub fn unget(&mut self, t: Token) {
        if let Some(last) = self.files.last_mut() {
            last.unget(t);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testsuite::load_testdata;

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
