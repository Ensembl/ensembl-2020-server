use crate::interp::commandsets::{ Command, CommandSchema, CommandType, CommandTrigger };
use crate::model::Register;
use crate::generate::{ Instruction, InstructionSuperType };
use serde_cbor::Value as CborValue;

pub struct BuiltinCommandType {
    supertype: InstructionSuperType,
    values: usize,
    ctor: Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>
}

impl BuiltinCommandType {
    pub fn new(supertype: InstructionSuperType, values: usize, ctor: Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>) -> BuiltinCommandType {
        BuiltinCommandType {
            supertype, values, ctor
        }
    }
}

impl CommandType for BuiltinCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: self.values,
            trigger: CommandTrigger::Instruction(self.supertype)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        (self.ctor)(&it.regs)
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        (self.ctor)(&(0..self.values).map(|x| Register::deserialize(&value[x])).collect::<Result<Vec<_>,String>>()?)
    }
}
