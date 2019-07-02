use std::collections::{ HashSet, HashMap };

use super::charstream::CharStream;

struct OpRegistryLen {
    len: usize,
    set: HashSet::<String>
}

impl OpRegistryLen {
    fn new(len: usize) -> OpRegistryLen {
        OpRegistryLen {
            len,
            set: HashSet::new()
        }
    }

    fn contains(&self, cs: &mut CharStream) -> bool {
        let s = cs.peek(self.len);
        self.set.contains(&s)
    }

    fn add(&mut self, op: &str) {
        self.set.insert(op.to_string());
    }
}

pub struct OpRegistry {
    lens: HashMap<usize,OpRegistryLen>,
    starts: HashMap<char,HashSet<usize>>
}

impl OpRegistry {
    pub fn new() -> OpRegistry {
        OpRegistry {
            lens: HashMap::new(),
            starts: HashMap::new()
        }
    }

    pub fn contains(&self, cs: &mut CharStream) -> Option<String> {
        if let Some(start) = cs.peek(1).chars().next() {
            if let Some(lens) = self.starts.get(&start) {
                for len in lens {
                    if self.lens.get(len).unwrap().contains(cs) {
                        return Some(cs.advance(*len));
                    }
                }
            }
        }
        None
    }

    pub fn add(&mut self, op: &str) {
        let len = op.len();
        if let Some(start) = op.chars().next() {
            let r = self.lens.entry(len).or_insert_with(|| OpRegistryLen::new(len));
            r.add(op);
            self.starts.entry(start).or_insert_with(|| HashSet::new()).insert(len);
        }
    }
}