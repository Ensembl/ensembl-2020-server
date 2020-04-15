use super::context::InterpContext;
use crate::generate::{ Instruction, InstructionSuperType };
use serde_cbor::Value as CborValue;

pub struct CommandSchema {
    pub opcode: u8,
    pub values: usize,
    pub instructions: Vec<InstructionSuperType>,
    pub commands: Vec<String>
}

pub trait CommandType {
    fn get_schema(&self) -> CommandSchema;
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String>;
    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String>;
}

pub trait Command {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String>;
    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Err("unimplemented".to_string())
    }
}
