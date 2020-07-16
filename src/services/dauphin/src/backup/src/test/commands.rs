/* 
 *  This is the default license template.
 *  
 *  File: commands.rs
 *  Author: dan
 *  Copyright (c) 2020 dan
 *  
 *  To edit this license information: Press Ctrl+Shift+P and press 'Create new License Template...'.
 */

use std::cell::RefCell;
use std::rc::Rc;
use dauphin_interp_common::common::{ CommandDeserializer, InterpCommand, Identifier };
use dauphin_interp_common::interp::{ InterpContext };
use dauphin_compile::model::{ Instruction };
use dauphin_compile::model::{ Command, CommandSchema, CommandTrigger, CommandType };
use serde_cbor::Value as CborValue;

pub struct FakeInterpCommand(Rc<RefCell<u32>>,u32);

impl InterpCommand for FakeInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        *self.0.borrow_mut() = self.1;
        Ok(())
    }
}

pub struct FakeDeserializer(pub Rc<RefCell<u32>>,pub u32);

impl CommandDeserializer for FakeDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((self.1,0))) }
    fn deserialize(&self, opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(FakeInterpCommand(self.0.clone(),self.1)))
    }
}

pub fn fake_trigger(name: &str) -> CommandTrigger {
    CommandTrigger::Command(Identifier::new("fake",name))
}

struct FakeCommandType(String);

impl CommandType for FakeCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            trigger: fake_trigger(&self.0),
            values: 0
        }
    }

    fn from_instruction(&self, _it: &Instruction) -> Result<Box<dyn Command>,String> {
        Err(format!("unable"))
    }
}

pub fn fake_command(name: &str) -> impl CommandType {
    FakeCommandType(name.to_string())
}
