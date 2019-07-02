pub trait CharSource {
    fn next(&mut self) -> Option<char>;
}

pub struct StringCharSource {
    data: Vec<char>,
    index: usize
}

impl StringCharSource {
    pub fn new(data: String) -> StringCharSource {
        StringCharSource { data: data.chars().collect(), index: 0 }
    }
}

impl CharSource for StringCharSource {
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
