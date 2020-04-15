use crate::interp::command::{ Command, CommandSchema, CommandType };
use crate::model::Register;
use crate::generate::{ Instruction, InstructionSuperType };
use serde_cbor::Value as CborValue;

pub struct BuiltinCommandType {
    supertype: InstructionSuperType,
    opcode: u8,
    values: usize,
    ctor: Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>
}

impl BuiltinCommandType {
    pub fn new(supertype: InstructionSuperType, opcode: u8, values: usize, ctor: Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>) -> BuiltinCommandType {
        BuiltinCommandType {
            supertype, opcode, values, ctor
        }
    }
}

impl CommandType for BuiltinCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            opcode: self.opcode,
            values: self.values,
            instructions: vec![self.supertype.clone()],
            commands: vec![]
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        (self.ctor)(&it.regs)
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        (self.ctor)(&(0..self.values).map(|x| Register::deserialize(&value[x])).collect::<Result<Vec<_>,String>>()?)
    }
}
