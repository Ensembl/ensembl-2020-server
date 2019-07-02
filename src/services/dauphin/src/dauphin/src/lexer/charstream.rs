use std::collections::VecDeque;

use super::charsource::{ CharSource, StringCharSource };

pub struct CharStream {
    source: Box<CharSource>,
    pending: VecDeque<char>,
    name: String,
    finished: bool,
    line: u32,
    column: u32
}

impl CharStream {
    pub fn new(src: Box<CharSource>, name: &str) -> CharStream {
        CharStream {
            source: src,
            pending: VecDeque::new(),
            finished: false,
            line: 1,
            column: 1,
            name: name.to_string()
        }
    }

    fn populate(&mut self, num: usize) {
        while !self.finished && num > self.pending.len() {
            if let Some(c) = self.source.next() {
                self.pending.push_back(c);
            } else {
                self.finished = true;
            }
        }
    }

    pub fn peek(&mut self, num: usize) -> String {
        self.populate(num);
        let mut out = String::new();
        for i in 0..num.min(self.pending.len()) {
            out.push(self.pending[i]);
        }
        out
    }

    pub fn advance(&mut self, num: usize) -> String {
        self.populate(num);
        let mut out = String::new();
        for _i in 0..num {
            if let Some(c) = self.pending.pop_front() {
                if c == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                out.push(c);
            }
        }
        out
    }

    pub fn position(&self) -> (u32,u32) {
        (self.line,self.column)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn charstream_smoke() {
        let s = "hello, world!\n";
        let mut cs = CharStream::new(Box::new(StringCharSource::new(s.to_string())),"name");
        assert_eq!("he",cs.peek(2));
        assert_eq!("h",cs.peek(1));
        assert_eq!("hel",cs.peek(3));
        assert_eq!("he",cs.advance(2));
        assert_eq!("ll",cs.peek(2));
        assert_eq!("l",cs.peek(1));
        assert_eq!("llo",cs.peek(3));
        assert_eq!((1,3),cs.position());
        assert_eq!("llo",cs.peek(3));
        assert_eq!("llo",cs.advance(3));
        assert_eq!((1,6),cs.position());
        assert_eq!(", world!\n",cs.advance(100));
        assert_eq!("",cs.advance(100));
        assert_eq!((2,1),cs.position());
        assert_eq!("name",cs.name());
    }
}