use std::cell::{ Ref, RefCell, RefMut };
use std::rc::Rc;
use std::collections::HashMap;
use std::iter::{ Iterator };
use crate::model::Register;
use super::registers::RegisterFile;
use super::supercow::{ SuperCow, SuperCowCommit };
use super::value::{ InterpNatural, InterpValueData, ReadOnlyValues, ReadWriteValues };

pub struct InterpContext {
    registers: RegisterFile
}

impl InterpContext {
    pub fn new() -> InterpContext {
        InterpContext {
            registers: RegisterFile::new()
        }
    }

    pub fn registers(&mut self) -> &mut RegisterFile { &mut self.registers }
}
