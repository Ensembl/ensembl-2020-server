use std::collections::{ HashSet, HashMap };

use super::charsource::CharSource;

struct InlineTokensLen {
    len: usize,
    set: HashSet::<String>
}

impl InlineTokensLen {
    fn new(len: usize) -> InlineTokensLen {
        InlineTokensLen {
            len,
            set: HashSet::new()
        }
    }

    fn contains(&self, cs: &mut dyn CharSource) -> bool {
        let s = cs.peek(self.len);
        self.set.contains(&s)
    }

    fn add(&mut self, op: &str) {
        self.set.insert(op.to_string());
    }
}

pub struct InlineTokens {
    lens: HashMap<usize,InlineTokensLen>,
    starts: HashMap<char,Vec<i32>>
}

impl InlineTokens {
    pub fn new() -> InlineTokens {
        InlineTokens {
            lens: HashMap::new(),
            starts: HashMap::new()
        }
    }

    fn check_inline(&self, c: &str) -> Result<(),String> {
        /* operators cannot contain slash-star, slash-slash, semicolon */
        for b in &vec!["//","/*",";"] {
            if c.contains(b) {
                return Err(format!("operator '{}' invalid, cannot contain '{}'",c,b));
            }
        }
        /* operators cannot contain whitespace */
        for c in c.chars() {
            if c.is_whitespace() {
                return Err(format!("operator '{}' invalid, cannot contain whitespace",c));
            }
        }
        Ok(())
    }

    pub fn contains(&self, cs: &mut dyn CharSource) -> Option<String> {
        if let Some(start) = cs.peek(1).chars().next() {
            if let Some(lens) = self.starts.get(&start) {
                for len in lens {
                    let len = *len as usize;
                    if self.lens.get(&len).unwrap().contains(cs) {
                        return Some(cs.advance(len));
                    }
                }
            }
        }
        None
    }

    pub fn add(&mut self, op: &str) -> Result<(),String> {
        self.check_inline(op)?;
        let len = op.len();
        if let Some(start) = op.chars().next() {
            let r = self.lens.entry(len).or_insert_with(|| InlineTokensLen::new(len));
            r.add(op);
            let lens = self.starts.entry(start).or_insert_with(|| Vec::new());
            lens.push(len as i32);
            lens.sort_by_key(|k| -k);
        }
        Ok(())
    }
}