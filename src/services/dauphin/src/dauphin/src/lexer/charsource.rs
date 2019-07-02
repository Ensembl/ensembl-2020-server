pub trait CharSource {
    fn name(&self) -> &str;
    fn next(&mut self) -> Option<char>;
    fn peek(&mut self, num: usize) -> String;
    fn advance(&mut self, num: usize) -> String;
}

pub struct StringCharSource {
    name: String,
    data: Vec<char>,
    index: usize
}

impl StringCharSource {
    pub fn new(name: &str, data: String) -> StringCharSource {
        StringCharSource { name: name.to_string(), data: data.chars().collect(), index: 0 }
    }
}

impl CharSource for StringCharSource {
    fn name(&self) -> &str { &self.name }

    fn peek(&mut self, num: usize) -> String {
        let c : Vec<char> = self.data[self.index..(self.index+num).min(self.data.len())].into();
        c.into_iter().collect()
    }

    fn advance(&mut self, num: usize) -> String {
        let out = self.peek(num);
        self.index += out.len();
        out
    }

    fn next(&mut self) -> Option<char> {
        if self.index < self.data.len() {
            let out = self.data[self.index];
            self.index += 1;
            Some(out)
        } else {
            None
        }
    }
}

pub struct LocatedCharSource {
    cs: Box<CharSource>,
    line: u32,
    col: u32
}

impl LocatedCharSource {
    pub fn new(cs: Box<CharSource>) -> LocatedCharSource {
        LocatedCharSource {
            cs, line: 1, col: 1
        }
    }

    pub fn position(&self) -> (u32,u32) {
        (self.line,self.col)
    }
}

impl CharSource for LocatedCharSource {
    fn name(&self) -> &str { self.cs.name() }

    fn peek(&mut self, num: usize) -> String {
        self.cs.peek(num)
    }

    fn advance(&mut self, num: usize) -> String {
        let out = self.cs.advance(num);
        for c in out.chars() {
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        out
    }

    fn next(&mut self) -> Option<char> {
        let out = self.advance(1);
        out.chars().next()
    }
}
