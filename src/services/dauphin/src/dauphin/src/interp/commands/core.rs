use super::super::context::{InterpContext };
//use super::super::value::InterpValueData;
use super::super::value::InterpValueData;
use super::super::command::Command;
use crate::model::Register;
use super::assign::{ blit, blit_expanded, blit_runs };

pub struct NilCommand(pub(crate) Register);

// XXX read is coerce
impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValueData::Empty);
        Ok(())
    }
}

pub struct NumberConstCommand(pub(crate) Register,pub(crate) f64);

impl Command for NumberConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValueData::Numbers(vec![self.1]));
        Ok(())
    }
}

pub struct ConstCommand(pub(crate) Register,pub(crate) Vec<usize>);

impl Command for ConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValueData::Indexes(self.1.to_vec()));
        Ok(())
    }
}

pub struct BooleanConstCommand(pub(crate) Register,pub(crate) bool);

impl Command for BooleanConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValueData::Boolean(vec![self.1]));
        Ok(())
    }
}

pub struct StringConstCommand(pub(crate) Register,pub(crate) String);

impl Command for StringConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValueData::Strings(vec![self.1.to_string()]));
        Ok(())
    }
}

pub struct BytesConstCommand(pub(crate) Register,pub(crate) Vec<u8>);

impl Command for BytesConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValueData::Bytes(vec![self.1.to_vec()]));
        Ok(())
    }
}

pub struct CopyCommand(pub(crate) Register,pub(crate) Register);

impl Command for CopyCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().copy(&self.0,&self.1);
        Ok(())
    }
}

pub struct AppendCommand(pub(crate) Register,pub(crate) Register);

impl Command for AppendCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1).borrow().get_shared()?;
        let mut dstr = registers.get(&self.0);
        let mut dst = dstr.borrow_mut().get_exclusive()?;
        registers.write(&self.0,blit(dst,&src,None)?);
        Ok(())
    }
}

pub struct LengthCommand(pub(crate) Register,pub(crate) Register);

impl Command for LengthCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let len = registers.get(&self.1).borrow().get_shared()?.len();
        registers.write(&self.0,InterpValueData::Indexes(vec![len]));
        Ok(())
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
        registers.write(&self.0,InterpValueData::Indexes(dst));
        Ok(())
    }
}

pub struct AtCommand(pub(crate) Register,pub(crate) Register);

impl Command for AtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = vec![];
        for i in 0..src.len() {
            dst.push(i);
        }
        registers.write(&self.0,InterpValueData::Indexes(dst));
        Ok(())
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
        registers.write(&self.0,InterpValueData::Boolean(dst));
        Ok(())
    }
}

pub struct FilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for FilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let filter = registers.get_boolean(&self.2)?;
        let src = registers.get(&self.1);
        let mut dstr = registers.get(&self.0);
        let src = src.borrow().get_shared()?;
        let mut dst = dstr.borrow_mut().get_exclusive()?;
        registers.write(&self.0,blit_expanded(dst,&src,&filter)?);
        Ok(())
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
        registers.write(&self.0,InterpValueData::Indexes(dst));
        Ok(())
    }
}

pub struct SeqFilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register, pub(crate) Register);

impl Command for SeqFilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1);
        let start = registers.get_indexes(&self.2)?;
        let len = registers.get_indexes(&self.3)?;
        let dst = registers.get(&self.0);
        let src = src.borrow().get_shared()?;
        let dst = dst.borrow_mut().get_exclusive()?;
        registers.write(&self.0,blit_runs(dst,&src,&start,&len)?);
        Ok(())
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
        registers.write(&self.0,InterpValueData::Indexes(dst));
        Ok(())
    }
}
