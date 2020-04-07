use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::model::Register;
use super::supercow::{ SuperCow, SuperCowCommit };
use super::value::{ InterpValueData, InterpValueNumbers, InterpValueIndexes, InterpValueBoolean, InterpValueStrings, InterpValueBytes };

pub struct RegisterFile {
    values: RefCell<HashMap<Register,Rc<RefCell<SuperCow<InterpValueData>>>>>,
    commits: Vec<Rc<RefCell<dyn SuperCowCommit>>>
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        let mut out = RegisterFile {
            values: RefCell::new(HashMap::new()),
            commits: Vec::new()
        };
        out
    }

    pub fn get(&self, register: &Register) -> Rc<RefCell<SuperCow<InterpValueData>>> {
        self.values.borrow_mut().entry(*register).or_insert_with(|| 
            Rc::new(RefCell::new(SuperCow::new(InterpValueData::Empty,|x| { x.copy() })))
        ).clone()
    }

    pub fn write(&mut self, register: &Register, value: InterpValueData) {
        let cow = self.get(register);
        cow.borrow_mut().set(value);
        self.commits.push(cow);
    }

    pub fn commit(&mut self) {
        for mut commit in self.commits.drain(..) {
            commit.borrow_mut().commit();
        }
    }

    pub fn copy(&mut self, dst: &Register, src: &Register) -> Result<(),String> {
        if src == dst { return Ok(()); }
        let src = self.get(src);
        let dst = self.get(dst);
        dst.borrow_mut().copy(&src.borrow());
        Ok(())
    }

    pub fn get_numbers(&self, register: &Register) -> Result<InterpValueNumbers,String> {
        InterpValueData::to_rc_numbers(&self.get(register).borrow().get_shared()?)
    }

    pub fn take_numbers(&mut self, register: &Register) -> Result<Vec<f64>,String> {
        Ok(self.get(register).borrow_mut().get_exclusive()?.to_numbers()?)
    }

    pub fn get_indexes(&self, register: &Register) -> Result<InterpValueIndexes,String> {
        InterpValueData::to_rc_indexes(&self.get(register).borrow().get_shared()?)
    }

    pub fn take_indexes(&mut self, register: &Register) -> Result<Vec<usize>,String> {
        Ok(self.get(register).borrow_mut().get_exclusive()?.to_indexes()?)
    }

    pub fn get_boolean(&self, register: &Register) -> Result<InterpValueBoolean,String> {
        InterpValueData::to_rc_boolean(&self.get(register).borrow().get_shared()?)
    }

    pub fn take_boolean(&mut self, register: &Register) -> Result<Vec<bool>,String> {
        Ok(self.get(register).borrow_mut().get_exclusive()?.to_boolean()?)
    }

    pub fn get_strings(&self, register: &Register) -> Result<InterpValueStrings,String> {
        InterpValueData::to_rc_strings(&self.get(register).borrow().get_shared()?)
    }

    pub fn take_string(&mut self, register: &Register) -> Result<Vec<String>,String> {
        Ok(self.get(register).borrow_mut().get_exclusive()?.to_strings()?)
    }

    pub fn get_bytes(&self, register: &Register) -> Result<InterpValueBytes,String> {
        InterpValueData::to_rc_bytes(&self.get(register).borrow().get_shared()?)
    }

    pub fn take_bytes(&mut self, register: &Register) -> Result<Vec<Vec<u8>>,String> {
        Ok(self.get(register).borrow_mut().get_exclusive()?.to_bytes()?)
    }
}
