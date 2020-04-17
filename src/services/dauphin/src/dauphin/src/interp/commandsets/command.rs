use std::fmt;
use crate::interp::context::InterpContext;
use crate::generate::{ Instruction, InstructionSuperType };
use serde_cbor::Value as CborValue;

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub enum CommandTrigger {
    Instruction(InstructionSuperType),
    Command(String)
}

impl fmt::Display for CommandTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandTrigger::Command(cmd) => write!(f,"{}",cmd),
            CommandTrigger::Instruction(instr) => write!(f,"builtin({:?})",instr)
        }
    }
}

pub struct CommandSchema {
    pub values: usize,
    pub trigger: CommandTrigger
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
