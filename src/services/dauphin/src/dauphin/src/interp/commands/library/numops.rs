use crate::interp::context::{InterpContext };
use crate::interp::InterpValue;
use crate::model::Register;
use crate::interp::commandsets::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet };
use crate::generate::Instruction;
use serde_cbor::Value as CborValue;

#[derive(Copy,Clone)]
pub(crate) enum InterpBinBoolOp {
    Lt,
    LtEq,
    Gt,
    GtEq
}

impl InterpBinBoolOp {
    fn evaluate(&self, a: f64, b: f64) -> bool {
        match self {
            InterpBinBoolOp::Lt => a < b,
            InterpBinBoolOp::LtEq => a <= b,
            InterpBinBoolOp::Gt => a > b,
            InterpBinBoolOp::GtEq => a >= b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpBinBoolOp::Lt => "lt",
            InterpBinBoolOp::LtEq => "lteq",
            InterpBinBoolOp::Gt => "gt",
            InterpBinBoolOp::GtEq => "gteq"
        }
    }
}

pub struct InterpBinBoolCommandType(InterpBinBoolOp);

impl CommandType for InterpBinBoolCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(self.0.name().to_string())
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }
}

pub struct InterpBinBoolCommand(pub(crate) InterpBinBoolOp, pub(crate) Register,pub(crate) Register,pub(crate) Register);

impl Command for InterpBinBoolCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = registers.get_numbers(&self.2)?;
        let b = &registers.get_numbers(&self.3)?;
        let mut c = vec![];
        let b_len = b.len();
        for (i,a_val) in a.iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        registers.write(&self.1,InterpValue::Boolean(c));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()])
    }
}

pub(super) fn library_numops_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("lt",5,InterpBinBoolCommandType(InterpBinBoolOp::Lt))?;
    set.push("lteq",6,InterpBinBoolCommandType(InterpBinBoolOp::LtEq))?;
    set.push("gt",7,InterpBinBoolCommandType(InterpBinBoolOp::Gt))?;
    set.push("gteq",8,InterpBinBoolCommandType(InterpBinBoolOp::GtEq))?;
    Ok(())
}
