use super::super::context::{InterpContext };
use super::super::value::InterpValueData;
use super::super::command::Command;
use crate::model::Register;
use super::assign::{ blit, blit_expanded, blit_runs };

pub struct NilCommand(pub(crate) Register);

impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write_empty(&self.0);
        Ok(())
    }
}

pub struct NumberConstCommand(pub(crate) Register,pub(crate) f64);

impl Command for NumberConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write_numbers(&self.0)?.borrow_mut().push(self.1);
        Ok(())
    }
}

pub struct ConstCommand(pub(crate) Register,pub(crate) Vec<usize>);

impl Command for ConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().set_indexes(&self.0,self.1.to_vec())?;
        Ok(())
    }
}

pub struct BooleanConstCommand(pub(crate) Register,pub(crate) bool);

impl Command for BooleanConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write_boolean(&self.0)?.borrow_mut().push(self.1);
        Ok(())
    }
}

pub struct StringConstCommand(pub(crate) Register,pub(crate) String);

impl Command for StringConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write_strings(&self.0)?.borrow_mut().push(self.1.to_string());
        Ok(())
    }
}

pub struct BytesConstCommand(pub(crate) Register,pub(crate) Vec<u8>);

impl Command for BytesConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write_bytes(&self.0)?.borrow_mut().push(self.1.to_vec());
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
        let src = registers.get(&self.1);
        let mut dstr = registers.get(&self.0);
        let src = src.read()?;
        let mut dst = dstr.modify()?;
        blit(&mut dst,&src,None)?;
        drop(dst);
        registers.add_commit(dstr);
        Ok(())
    }
}

pub struct LengthCommand(pub(crate) Register,pub(crate) Register);

impl Command for LengthCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let len = registers.get(&self.1).read()?.len();
        registers.set_indexes(&self.0,vec![len])?;
        Ok(())
    }
}

pub struct AddCommand(pub(crate) Register,pub(crate) Register);

impl Command for AddCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.read_indexes(&self.1)?;
        let dst = registers.modify_indexes(&self.0)?;
        let src = src.borrow();
        let mut dst = dst.borrow_mut();
        let src_len = (&src).len();
        for i in 0..dst.len() {
            dst[i] += src[i%src_len];
        }
        Ok(())
    }
}

pub struct AtCommand(pub(crate) Register,pub(crate) Register);

impl Command for AtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.read_indexes(&self.1)?;
        let dst = registers.write_indexes(&self.0)?;
        let src = src.borrow();
        let mut dst = dst.borrow_mut();
        for i in 0..src.len() {
            dst[i] = i;
        }
        Ok(())
    }
}

pub struct NumEqCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for NumEqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src1 = registers.read_indexes(&self.1)?;
        let src2 = registers.read_indexes(&self.2)?;
        let dst = registers.write_boolean(&self.0)?;
        let src1 = src1.borrow();
        let src2 = src2.borrow();
        let mut dst = dst.borrow_mut();
        let src2len = src2.len();
        for i in 0..src1.len() {
            dst[i] = src1[i] == src2[i%src2len];
        }
        Ok(())
    }
}

pub struct FilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for FilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let filter = registers.read_boolean(&self.2)?;
        let src = registers.get(&self.1);
        let mut dst = registers.get(&self.0);
        let src = src.read()?;
        let mut dst = dst.write();
        blit_expanded(&mut dst,&src,&filter.borrow())?;
        Ok(())
    }
}

pub struct RunCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for RunCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let start = registers.read_indexes(&self.1)?;
        let len = registers.read_indexes(&self.2)?;
        let dst = registers.write_indexes(&self.0)?;
        let start = start.borrow();
        let len = len.borrow();
        let startlen = start.len();
        let lenlen = len.len();
        let mut dst = dst.borrow_mut();
        for i in 0..startlen {
            for j in 0..len[i%lenlen] {
                dst.push(start[i]+j);
            }
        }
        Ok(())
    }
}

pub struct SeqFilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register, pub(crate) Register);

impl Command for SeqFilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1);
        let start = registers.read_indexes(&self.2)?;
        let len = registers.read_indexes(&self.3)?;
        let mut dst = registers.get(&self.0);
        let start = start.borrow();
        let len = len.borrow();
        let src = src.read()?;
        let mut dst = dst.write();
        blit_runs(&mut dst,&src,&start,&len)
    }
}

pub struct SeqAtCommand(pub(crate) Register,pub(crate) Register);

impl Command for SeqAtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.read_indexes(&self.1)?;
        let dst = registers.write_indexes(&self.0)?;
        let src = src.borrow();
        let mut dst = dst.borrow_mut();
        for i in 0..src.len() {
            for j in 0..src[i] {
                dst.push(j);
            }
        }
        Ok(())
    }
}
