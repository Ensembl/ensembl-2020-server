use crate::interp::context::{InterpContext };
use crate::interp::InterpValue;
use crate::interp::commandsets::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet };
use crate::model::Register;
use crate::generate::{ Instruction, InstructionType, InstructionSuperType };
use serde_cbor::Value as CborValue;

// XXX factor
macro_rules! force_branch {
    ($value:expr,$ty:ident,$branch:ident) => {
        if let $ty::$branch(v) = $value {
            Ok(v)
        } else {
            Err("Cannot extract".to_string())
        }?
    };
}

pub struct NumberConstCommandType();

impl CommandType for NumberConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::NumberConst),
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumberConstCommand(it.regs[0],force_branch!(it.itype,InstructionType,NumberConst))))
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumberConstCommand(Register::deserialize(&value[0])?,force_branch!(value[1],CborValue,Float))))
    }
}

pub struct NumberConstCommand(pub(crate) Register,pub(crate) f64);

impl Command for NumberConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Numbers(vec![self.1]));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),CborValue::Float(self.1)])
    }
}

pub struct ConstCommandType();

impl CommandType for ConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::Const)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(ConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,Const).to_vec())))
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        let v = force_branch!(&value[1],CborValue,Array);
        let v = v.iter().map(|x| { Ok(*force_branch!(x,CborValue,Integer) as usize) }).collect::<Result<Vec<usize>,String>>()?;
        Ok(Box::new(ConstCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct ConstCommand(pub(crate) Register,pub(crate) Vec<usize>);

impl Command for ConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Indexes(self.1.to_vec()));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let v = self.1.iter().map(|x| CborValue::Integer(*x as i128)).collect();
        Ok(vec![self.0.serialize(),CborValue::Array(v)])
    }
}

pub struct BooleanConstCommandType();

impl CommandType for BooleanConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::BooleanConst)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BooleanConstCommand(it.regs[0],force_branch!(it.itype,InstructionType,BooleanConst))))
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BooleanConstCommand(Register::deserialize(&value[0])?,force_branch!(value[1],CborValue,Bool))))
    }
}

pub struct BooleanConstCommand(pub(crate) Register,pub(crate) bool);

impl Command for BooleanConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Boolean(vec![self.1]));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),CborValue::Bool(self.1)])
    }
}

pub struct StringConstCommandType();

impl CommandType for StringConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::StringConst)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(StringConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,StringConst).to_string())))
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        let v = force_branch!(&value[1],CborValue,Text).to_string();
        Ok(Box::new(StringConstCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct StringConstCommand(pub(crate) Register,pub(crate) String);

impl Command for StringConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Strings(vec![self.1.to_string()]));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),CborValue::Text(self.1.to_string())])
    } 
}

pub struct BytesConstCommandType();

impl CommandType for BytesConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::BytesConst)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BytesConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,BytesConst).to_vec())))
    }

    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        let v = force_branch!(&value[1],CborValue,Bytes).to_vec();
        Ok(Box::new(BytesConstCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct BytesConstCommand(pub(crate) Register,pub(crate) Vec<u8>);

impl Command for BytesConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Bytes(vec![self.1.to_vec()]));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),CborValue::Bytes(self.1.to_vec())])
    } 
}

pub(super) fn const_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("number",0,NumberConstCommandType())?;
    set.push("const",1,ConstCommandType())?;
    set.push("boolean",2,BooleanConstCommandType())?;
    set.push("string",3,StringConstCommandType())?;
    set.push("bytes",4,BytesConstCommandType())?;
    Ok(())
}