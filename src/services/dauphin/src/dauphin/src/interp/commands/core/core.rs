use crate::interp::context::{InterpContext };
use crate::interp::InterpValue;
use crate::interp::command::{ Command, CommandSchema, CommandType };
use crate::model::Register;
use crate::interp::commands::assign::{ blit, blit_expanded, blit_runs };
use crate::generate::{ Instruction, InstructionSuperType };
use serde_cbor::Value as CborValue;

// XXX read is coerce

struct InstrPlainCommand(InstructionSuperType,u8,usize,Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>);

impl CommandType for InstrPlainCommand {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            opcode: self.1,
            values: self.2,
            instructions: vec![self.0.clone()],
            commands: vec![]
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        (self.3)(&it.regs)
    }

    fn deserialise(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        (self.3)(&(0..self.2).map(|x| Register::deserialize(&value[x])).collect::<Result<Vec<_>,String>>()?)
    }
}

fn core_commands() {
    let nil_command = InstrPlainCommand(InstructionSuperType::Nil,0,1,Box::new(|x| Ok(Box::new(NilCommand(x[0])))));
    /* 1-5 used in consts */
    let copy_command = InstrPlainCommand(InstructionSuperType::Copy,6,2,Box::new(|x| Ok(Box::new(CopyCommand(x[0],x[1])))));
    let append_command = InstrPlainCommand(InstructionSuperType::Append,7,2,Box::new(|x| Ok(Box::new(AppendCommand(x[0],x[1])))));
    let length_command = InstrPlainCommand(InstructionSuperType::Length,8,2,Box::new(|x| Ok(Box::new(LengthCommand(x[0],x[1])))));
    let add_command = InstrPlainCommand(InstructionSuperType::Add,9,2,Box::new(|x| Ok(Box::new(AddCommand(x[0],x[1])))));
    let numeq_command = InstrPlainCommand(InstructionSuperType::Add,10,3,Box::new(|x| Ok(Box::new(NumEqCommand(x[0],x[1],x[2])))));
    let filter_command = InstrPlainCommand(InstructionSuperType::Add,11,3,Box::new(|x| Ok(Box::new(FilterCommand(x[0],x[1],x[2])))));
    let run_command = InstrPlainCommand(InstructionSuperType::Add,12,3,Box::new(|x| Ok(Box::new(RunCommand(x[0],x[1],x[2])))));
    let seqfilter_command = InstrPlainCommand(InstructionSuperType::Add,13,4,Box::new(|x| Ok(Box::new(SeqFilterCommand(x[0],x[1],x[2],x[3])))));
    let seqat_command = InstrPlainCommand(InstructionSuperType::Add,14,2,Box::new(|x| Ok(Box::new(SeqAtCommand(x[0],x[1])))));
}

pub struct NilCommand(pub(crate) Register);

impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Empty);
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise()])
    }
}

pub struct CopyCommand(pub(crate) Register,pub(crate) Register);

impl Command for CopyCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().copy(&self.0,&self.1)?;
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise()])
    }
}

pub struct AppendCommand(pub(crate) Register,pub(crate) Register);

impl Command for AppendCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1).borrow().get_shared()?;
        let dstr = registers.get(&self.0);
        let dst = dstr.borrow_mut().get_exclusive()?;
        registers.write(&self.0,blit(dst,&src,None)?);
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise()])
    }
}

pub struct LengthCommand(pub(crate) Register,pub(crate) Register);

impl Command for LengthCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let len = registers.get(&self.1).borrow().get_shared()?.len();
        registers.write(&self.0,InterpValue::Indexes(vec![len]));
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise()])
    }
}

pub struct AddCommand(pub(crate) Register,pub(crate) Register);

impl Command for AddCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = registers.take_indexes(&self.0)?;
        let src_len = (&src).len();
        for i in 0..dst.len() {
            dst[i] += src[i%src_len];
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise()])
    }
}

pub struct NumEqCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for NumEqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src1 = &registers.get_indexes(&self.1)?;
        let src2 = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        let src2len = src2.len();
        for i in 0..src1.len() {
            dst[i] = src1[i] == src2[i%src2len];
        }
        registers.write(&self.0,InterpValue::Boolean(dst));
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise(),self.2.serialise()])
    }
}

pub struct FilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for FilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let filter = registers.get_boolean(&self.2)?;
        let src = registers.get(&self.1);
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,blit_expanded(InterpValue::Empty,&src,&filter)?);
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise(),self.2.serialise()])
    }
}

pub struct RunCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for RunCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let start = &registers.get_indexes(&self.1)?;
        let len = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        let startlen = start.len();
        let lenlen = len.len();
        for i in 0..startlen {
            for j in 0..len[i%lenlen] {
                dst.push(start[i]+j);
            }
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise(),self.2.serialise()])
    }
}

pub struct SeqFilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register, pub(crate) Register);

impl Command for SeqFilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1);
        let start = registers.get_indexes(&self.2)?;
        let len = registers.get_indexes(&self.3)?;
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,blit_runs(InterpValue::Empty,&src,&start,&len)?);
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise(),self.2.serialise(),self.3.serialise()])
    }
}

pub struct SeqAtCommand(pub(crate) Register,pub(crate) Register);

impl Command for SeqAtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = vec![];
        for i in 0..src.len() {
            for j in 0..src[i] {
                dst.push(j);
            }
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialise(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialise(),self.1.serialise()])
    }
}
