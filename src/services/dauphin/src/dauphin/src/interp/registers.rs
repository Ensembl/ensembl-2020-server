use std::cell::{ Ref, RefCell, RefMut };
use std::rc::Rc;
use std::collections::HashMap;
use std::iter::{ Iterator };
use crate::model::Register;
use super::supercow::{ SuperCow, SuperCowCommit };
use super::value::{ InterpNatural, InterpValueData, ReadOnlyValues, ReadWriteValues };

pub struct RegisterFile {
    values: HashMap<Register,SuperCow<InterpValueData>>,
    commits: Vec<Box<dyn SuperCowCommit + 'static>>
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        let mut out = RegisterFile {
            values: HashMap::new(),
            commits: Vec::new()
        };
        out
    }

    fn get(&mut self, register: &Register) -> SuperCow<InterpValueData> {
        self.values.entry(*register).or_insert_with(|| SuperCow::new(|| { InterpValueData::Empty }, 
                                                    |x| { x.copy() },InterpValueData::Empty)).clone()
    }

    pub fn get_natural(&mut self, register: &Register) -> Result<InterpNatural,String> {
        Ok(self.get(register).read()?.get_natural())
    }

    pub fn commit(&mut self) {
        for mut commit in self.commits.drain(..) {
            commit.commit();
        }
    }

    pub fn copy(&mut self, dst: &Register, src: &Register) -> Result<(),String> {
        let src = self.get(src);
        let mut dst = self.get(dst);
        dst.copy(&src)
    }

    pub fn write_empty(&mut self, register: &Register) {
        let mut sc = self.get(register);
        sc.write();
        self.commits.push(Box::new(sc));
    }

    pub fn read_numbers(&mut self, register: &Register) -> Result<ReadOnlyValues<f64>,String> {
        self.get(register).read()?.read_numbers()
    }

    pub fn write_numbers(&mut self, register: &Register) -> Result<ReadWriteValues<f64>,String> {
        let mut sc = self.get(register);
        let out = sc.write().write_numbers();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn modify_numbers(&mut self, register: &Register) -> Result<ReadWriteValues<f64>,String> {
        let mut sc = self.get(register);
        let out = sc.modify()?.write_numbers();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn set_numbers(&mut self, register: &Register, value: Vec<f64>) -> Result<(),String> {
        let mut sc = self.get(register);
        sc.write().set_numbers(value)?;
        self.commits.push(Box::new(sc));
        Ok(())
    }

    pub fn read_indexes(&mut self, register: &Register) -> Result<ReadOnlyValues<usize>,String> {
        self.get(register).read()?.read_indexes()
    }

    pub fn write_indexes(&mut self, register: &Register) -> Result<ReadWriteValues<usize>,String> {
        let mut sc = self.get(register);
        let out = sc.write().write_indexes();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn modify_indexes(&mut self, register: &Register) -> Result<ReadWriteValues<usize>,String> {
        let mut sc = self.get(register);
        let out = sc.modify()?.write_indexes();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn set_indexes(&mut self, register: &Register, value: Vec<usize>) -> Result<(),String> {
        let mut sc = self.get(register);
        sc.write().set_indexes(value)?;
        self.commits.push(Box::new(sc));
        Ok(())
    }

    pub fn read_boolean(&mut self, register: &Register) -> Result<ReadOnlyValues<bool>,String> {
        self.get(register).read()?.read_boolean()
    }

    pub fn write_boolean(&mut self, register: &Register) -> Result<ReadWriteValues<bool>,String> {
        let mut sc = self.get(register);
        let out = sc.write().write_boolean();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn modify_boolean(&mut self, register: &Register) -> Result<ReadWriteValues<bool>,String> {
        let mut sc = self.get(register);
        let out = sc.modify()?.write_boolean();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn set_boolean(&mut self, register: &Register, value: Vec<bool>) -> Result<(),String> {
        let mut sc = self.get(register);
        sc.write().set_boolean(value)?;
        self.commits.push(Box::new(sc));
        Ok(())
    }

    pub fn read_strings(&mut self, register: &Register) -> Result<ReadOnlyValues<String>,String> {
        self.get(register).read()?.read_strings()
    }

    pub fn write_strings(&mut self, register: &Register) -> Result<ReadWriteValues<String>,String> {
        let mut sc = self.get(register);
        let out = sc.write().write_strings();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn modify_strings(&mut self, register: &Register) -> Result<ReadWriteValues<String>,String> {
        let mut sc = self.get(register);
        let out = sc.modify()?.write_strings();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn set_strings(&mut self, register: &Register, value: Vec<String>) -> Result<(),String> {
        let mut sc = self.get(register);
        sc.write().set_strings(value)?;
        self.commits.push(Box::new(sc));
        Ok(())
    }

    pub fn read_bytes(&mut self, register: &Register) -> Result<ReadOnlyValues<Vec<u8>>,String> {
        self.get(register).read()?.read_bytes()
    }

    pub fn write_bytes(&mut self, register: &Register) -> Result<ReadWriteValues<Vec<u8>>,String> {
        let mut sc = self.get(register);
        let out = sc.write().write_bytes();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn modify_bytes(&mut self, register: &Register) -> Result<ReadWriteValues<Vec<u8>>,String> {
        let mut sc = self.get(register);
        let out = sc.modify()?.write_bytes();
        self.commits.push(Box::new(sc));
        out
    }

    pub fn set_bytes(&mut self, register: &Register, value: Vec<Vec<u8>>) -> Result<(),String> {
        let mut sc = self.get(register);
        sc.write().set_bytes(value)?;
        self.commits.push(Box::new(sc));
        Ok(())
    }
}
