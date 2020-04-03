use std::cell::{ Ref, RefCell, RefMut };
use std::rc::Rc;
use std::collections::HashMap;
use std::iter::{ Iterator };
use crate::model::Register;
use super::registers::RegisterFile;
use super::supercow::{ SuperCow, SuperCowCommit };
use super::stream::{ Stream, StreamContents };
use super::value::{ InterpNatural, InterpValueData, ReadOnlyValues, ReadWriteValues };

pub struct InterpContext {
    registers: RegisterFile,
    stream: Stream
}

impl InterpContext {
    pub fn new() -> InterpContext {
        InterpContext {
            registers: RegisterFile::new(),
            stream: Stream::new()
        }
    }

    pub fn registers(&mut self) -> &mut RegisterFile { &mut self.registers }
    pub fn stream_add(&mut self, contents: StreamContents) { self.stream.add(contents); }
    pub fn stream_take(&mut self) -> Vec<StreamContents> { self.stream.take() }
}
