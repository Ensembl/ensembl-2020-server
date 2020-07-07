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

pub trait CharSource {
    fn to_string(&self) -> String;
    fn name(&self) -> &str;
    fn module(&self) -> &str;
    fn next(&mut self) -> Option<char>;
    fn peek(&mut self, num: usize) -> String;
    fn advance(&mut self, num: usize) -> String;
    fn retreat(&mut self, num: usize) -> String;
    fn pos(&self) -> usize;
}

pub struct StringCharSource {
    name: String,
    module: String,
    data: Vec<char>,
    index: usize
}

impl StringCharSource {
    pub fn new(name: &str, module: &str, data: String) -> StringCharSource {
        StringCharSource { name: name.to_string(), module: module.to_string(), data: data.chars().collect(), index: 0 }
    }
}

impl CharSource for StringCharSource {
    fn to_string(&self) -> String { self.data.iter().collect() }
    fn name(&self) -> &str { &self.name }
    fn module(&self) -> &str { &self.module }

    fn peek(&mut self, num: usize) -> String {
        let start = self.index;
        let end = (self.index+num).min(self.data.len());
        let c : Vec<char> = self.data[start..end].into();
        c.into_iter().collect()
    }

    fn advance(&mut self, num: usize) -> String {
        let out = self.peek(num);
        self.index += out.len();
        out
    }

    fn retreat(&mut self, num: usize) -> String {
        self.index -= num;
        self.peek(num)
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

    fn pos(&self) -> usize { self.index }
}

pub struct LocatedCharSource {
    cs: Box<dyn CharSource>,
    line: u32,
    col: u32
}

impl CharSource for LocatedCharSource {
    fn to_string(&self) -> String { self.cs.to_string() }
    fn name(&self) -> &str { self.cs.name() }
    fn module(&self) -> &str { self.cs.module() }

    fn peek(&mut self, num: usize) -> String {
        self.cs.peek(num)
    }

    fn advance(&mut self, num: usize) -> String {
        let out = self.cs.advance(num);
        for c in out.chars() {
            self.fwd_char(c);
        }
        out
    }

    fn retreat(&mut self, num: usize) -> String {
        let out = self.cs.retreat(num);
        for c in out.chars() {
            self.rev_char(c);
        }
        self.col = self.line_start_dist()+1;
        out
    }

    fn next(&mut self) -> Option<char> {
        let out = self.advance(1);
        out.chars().next()
    }

    fn pos(&self) -> usize { self.cs.pos() }
}

impl LocatedCharSource {
    pub fn new(cs: Box<dyn CharSource>) -> LocatedCharSource {
        LocatedCharSource {
            cs, line: 1, col: 1
        }
    }

    fn fwd_char(&mut self, c: char) {
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }

    fn line_start_dist(&mut self) -> u32 {
        if self.cs.pos() == 0 {
            return 0;
        }
        let cursor = self.cs.pos();
        while self.cs.pos() > 0 {
            self.cs.retreat(1);
            if self.cs.peek(1) == "\n" { break; }
        }
        let dist = cursor-self.cs.pos() - 1;
        self.cs.advance(dist+1);
        dist as u32
    }

    fn rev_char(&mut self, c: char) {
        if c == '\n' {
            self.line -= 1;
        }
    }

    pub fn position(&self) -> (u32,u32) {
        (self.line,self.col)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn start_test(start: usize) -> u32 {
                /*   012345 6 7890123 */
        let text = "\nABCDE\n\nFGHIJ\n";
        let c = StringCharSource::new("","",text.to_string());
        let mut lcs = LocatedCharSource::new(Box::new(c));
        lcs.advance(start);
        lcs.line_start_dist()
    }

    fn position_test(start: usize,retreat: usize) -> (u32,u32) {
                /*   012345 6 7890123 */
        let text = "\nABCDE\n\nFGHIJ\n";
        let c = StringCharSource::new("","",text.to_string());
        let mut lcs = LocatedCharSource::new(Box::new(c));
        lcs.advance(start);
        lcs.retreat(retreat);
        lcs.position()
    }

    #[test]
    fn test_line_start() {
        assert_eq!(start_test(0),0);
        assert_eq!(start_test(1),0);
        assert_eq!(start_test(2),1);
        assert_eq!(start_test(3),2);
        assert_eq!(start_test(4),3);
        assert_eq!(start_test(5),4);
        assert_eq!(start_test(6),5);
        assert_eq!(start_test(7),0);
        assert_eq!(start_test(8),0);
        assert_eq!(start_test(9),1);
        assert_eq!(start_test(10),2);
        assert_eq!(start_test(11),3);
        assert_eq!(start_test(12),4);
        assert_eq!(start_test(13),5);
    }

    #[test]
    fn test_line_reverse() {
        let pos = [(1,1),
                   (2,1),(2,2),(2,3),(2,4),(2,5),(2,6),
                   (3,1),
                   (4,1),(4,2),(4,3),(4,4),(4,5),(4,6)];
        for dest in 0..13 {
            for src in dest..13 {
                assert_eq!(position_test(src,src-dest),pos[dest]);
            }
        }
    }
}
