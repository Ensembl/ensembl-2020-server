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

pub struct InlineTokensSection {
    prefix: bool,
    lens: HashMap<usize,InlineTokensLen>,
    starts: HashMap<char,Vec<i32>>
}

impl InlineTokensSection {
    pub fn new(prefix: bool) -> InlineTokensSection {
        InlineTokensSection {
            prefix,
            lens: HashMap::new(),
            starts: HashMap::new()
        }
    }

    fn check_inline(&self, c: &str) -> Result<(),String> {
        /* cannot contain slash-star, slash-slash, semicolon */
        for b in &vec!["//","/*",";"] {
            if c.contains(b) {
                return Err(format!("operator '{}' invalid, cannot contain '{}'",c,b));
            }
        }
        /* cannot contain whitespace */
        for c in c.chars() {
            if c.is_whitespace() {
                return Err(format!("operator '{}' invalid, cannot contain whitespace",c));
            }
        }
        /* cannot begin with alphanumerics or be blank */
        if let Some(c) = c.chars().next() {
            if c.is_alphanumeric() || c == '_' {
                return Err("operator cannot begin with alphanumeric".to_string());
            }
        } else {
            return Err("operator cannot be blank".to_string());
        }
        /* cannot begin "," ")" "]": expression end? or "(" function start? */
        if let Some(c) = c.chars().next() {
            if c == ',' || c == ';' || c == ')' || c == ']' || c == '(' {
                return Err("operator cannot begin , ) ] or (".to_string());
            }
        }
        /* "." "?" "!" not valid followed by alphanumeric, not valid alone */
        let mut c = c.chars();
        if let Some(ch) = c.next() {
            if ch == '.' || ch == '?' || ch == '!' {
                if let Some(ch) = c.next() {
                    if ch.is_alphanumeric() || ch == '_' {
                        return Err("operator cannot be .?! followed by alphanumeric".to_string());
                    }
                } else {
                    return Err("operator cannot be .?!".to_string());
                }
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
        //self.check_inline(op)?;
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
            prefix: InlineTokensSection::new(true),
            normal: InlineTokensSection::new(false)
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

    pub fn add(&mut self, op: &str, prefix: bool) -> Result<(),String> {
        self.part_mut(prefix).add(op)
    }
}
