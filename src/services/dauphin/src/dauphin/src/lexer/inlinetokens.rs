use std::collections::{ HashSet, HashMap };

use super::charsource::CharSource;
use super::inlinecheck::check_inline;

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

    fn equal(&self, op: &str) -> bool {
        self.set.contains(op)
    }

    fn is_prefix_of(&self, op: &str) -> bool {
        self.set.iter().filter(|x| {
            if x as &str == op {
                false
            } else {
                op.starts_with(x as &str) || x.starts_with(op)
            }
        }).next().is_some()
    }

    fn add(&mut self, op: &str) {
        self.set.insert(op.to_string());
    }
}

pub struct InlineTokensSection {
    lens: HashMap<usize,InlineTokensLen>,
    starts: HashMap<char,Vec<i32>>
}

impl InlineTokensSection {
    pub fn new() -> InlineTokensSection {
        InlineTokensSection {
            lens: HashMap::new(),
            starts: HashMap::new()
        }
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

    pub fn equal(&self, op: &str) -> bool {
        if let Some(len) = self.lens.get(&op.len()) {
            len.equal(op)
        } else {
            false
        }
    }

    pub fn is_prefix_of(&self, op: &str) -> bool {
        if let Some(start) = op.chars().next() {
            if let Some(lens) = self.starts.get(&start) {
                for len in lens {
                    let len = *len as usize;
                    if self.lens.get(&len).unwrap().is_prefix_of(op) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn add(&mut self, op: &str) -> Result<(),String> {
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

pub struct InlineTokens {
    prefix: InlineTokensSection,
    normal: InlineTokensSection
}

impl InlineTokens {
    pub fn new() -> InlineTokens {
        InlineTokens {
            prefix: InlineTokensSection::new(),
            normal: InlineTokensSection::new()
        }
    }

    fn part(&self, prefix: bool) -> &InlineTokensSection {
        if prefix { &self.prefix } else { &self.normal }
    }

    fn part_mut(&mut self, prefix: bool) -> &mut InlineTokensSection {
        if prefix { &mut self.prefix } else { &mut self.normal }
    }

    pub fn contains(&self, cs: &mut dyn CharSource, prefix: bool) -> Option<String> {
        self.part(prefix).contains(cs)
    }

    pub fn equal(&self, op: &str, prefix: bool) -> bool {
        self.part(prefix).equal(op)
    }

    pub(in super) fn is_prefix_of(&self, op: &str) -> bool {
        self.part(false).is_prefix_of(op) || self.part(true).is_prefix_of(op)
    }

    pub fn add(&mut self, op: &str, prefix: bool) -> Result<(),String> {
        check_inline(&self,op,prefix)?;
        self.part_mut(prefix).add(op)
    }
}
