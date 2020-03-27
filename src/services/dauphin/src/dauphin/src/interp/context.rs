use std::collections::HashMap;
use std::iter::{ Iterator };
use crate::model::Register;
use super::value::InterpValue;

pub struct InterpContext {
    pending: Option<HashMap<Register,InterpValue>>,
    values: HashMap<Option<Register>,InterpValue>
}

impl InterpContext {
    pub fn new() -> InterpContext {
        let mut out = InterpContext {
            pending: None,
            values: HashMap::new()
        };
        out.values.insert(None,InterpValue::Empty);
        out
    }

    pub fn insert(&mut self, r: &Register, v: InterpValue) {
        if let Some(ref mut pending) = self.pending {
            pending.insert(*r,v);
        } else {
            self.values.insert(Some(*r),v);
        }
    }

    pub fn begin(&mut self) {
        self.pending = Some(HashMap::new());
    }

    pub fn commit(&mut self) {
        for (r,v) in self.pending.take().unwrap().drain() {
            self.insert(&r,v);
        }
    }

    pub fn get_mut<'a>(&'a mut self, r: &Register) -> &'a mut InterpValue { // XXX txn bug
        self.values.entry(Some(*r)).or_insert(InterpValue::Empty)
    }

    pub fn get<'a>(&'a self, r: &Register) -> &'a InterpValue {
        self.values.get(&Some(*r)).unwrap_or(self.values.get(&None).unwrap())
    }

    pub fn dump(&mut self) -> HashMap<Register,InterpValue> {
        self.values.drain().filter(|(k,_)| k.is_some()).map(|(k,v)| (k.unwrap(),v)).collect()
    }

    pub fn copy(&mut self, dst: &Register, src: &Register) {
        let v = self.values.get(&Some(*src)).unwrap_or(self.values.get(&None).unwrap());
        self.values.insert(Some(*dst),v.clone());
    }
}
