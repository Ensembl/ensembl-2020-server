/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::rc::Rc;
use crate::interp::InterpValue;
use crate::interp::{ Command, CommandSet, CommandSetId, InterpContext };
use crate::model::Register;
use crate::generate::InstructionSuperType;
use serde_cbor::Value as CborValue;
use super::commontype::BuiltinCommandType;
use crate::commands::common::polymorphic::arbitrate_type;
use super::consts::const_commands;

// XXX read is coerce

pub struct NilCommand(pub(crate) Register);

impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Empty);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize()])
    }
}

pub struct CopyCommand(pub(crate) Register,pub(crate) Register);

impl Command for CopyCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().copy(&self.0,&self.1)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }
}

pub struct AppendCommand(pub(crate) Register,pub(crate) Register);

fn append_typed<T>(dst: &mut Vec<T>, src: &Vec<T>) where T: Clone {
    dst.append(&mut src.clone());
}

fn append(dst: InterpValue, src: &Rc<InterpValue>) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&dst,src,false) {
        Ok(polymorphic!(dst,[src],natural,(|d,s| {
            append_typed(d,s)
        })))
    } else {
        Ok(dst)
    }
}

impl Command for AppendCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1).borrow().get_shared()?;
        let dstr = registers.get(&self.0);
        let dst = dstr.borrow_mut().get_exclusive()?;
        registers.write(&self.0,append(dst,&src)?);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
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

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
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

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }
}

pub struct ReFilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for ReFilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src : &[usize] = &registers.get_indexes(&self.1)?;
        let indexes : &[usize] = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        for x in indexes.iter() {
            dst.push(src[*x]);
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
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
            dst.push(src1[i] == src2[i%src2len]);
        }
        registers.write(&self.0,InterpValue::Boolean(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
    }
}

fn filter_typed<T>(dst: &mut Vec<T>, src: &[T], filter: &[bool]) where T: Clone {
    let filter_len = filter.len();
    for (i,value) in src.iter().enumerate() {
        if filter[i%filter_len] {
            dst.push(value.clone());
        }
    }
}

pub fn filter(src: &Rc<InterpValue>, filter_val: &[bool]) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&InterpValue::Empty,src,true) {
        Ok(polymorphic!(InterpValue::Empty,[src],natural,(|d,s| {
            filter_typed(d,s,filter_val)
        })))
    } else {
        Ok(InterpValue::Empty)
    }
}

pub struct FilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for FilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let filter_val = registers.get_boolean(&self.2)?;
        let src = registers.get(&self.1);
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,filter(&src,&filter_val)?);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
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

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
    }
}

pub struct AtCommand(pub(crate) Register, pub(crate) Register);

impl Command for AtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = vec![];
        for i in 0..src.len() {
            dst.push(i);
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }
}

fn seq_filter_typed<T>(dst: &mut Vec<T>, src: &[T], starts: &[usize], lens: &[usize]) where T: Clone {
    let starts_len = starts.len();
    let lens_len = lens.len();
    let src_len = src.len();
    for i in 0..starts_len {
        for j in 0..lens[i%lens_len] {
            dst.push(src[(starts[i]+j)%src_len].clone());
        }
    }
}

fn seq_filter(src: &Rc<InterpValue>, starts: &[usize], lens: &[usize]) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&InterpValue::Empty,src,true) {
        Ok(polymorphic!(InterpValue::Empty,[src],natural,(|d,s| {
            seq_filter_typed(d,s,starts,lens)
        })))
    } else {
        Ok(InterpValue::Empty)
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
        registers.write(&self.0,seq_filter(&src,&start,&len)?);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize(),self.3.serialize()])
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

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }
}

pub fn make_core() -> Result<CommandSet,String> {
    let set_id = CommandSetId::new("core",(0,0),0xD99E736DBD9EB7C5);
    let mut set = CommandSet::new(&set_id);
    const_commands(&mut set)?;
    set.push("nil",5,BuiltinCommandType::new(InstructionSuperType::Nil,1,Box::new(|x| Ok(Box::new(NilCommand(x[0]))))))?;
    set.push("copy",6,BuiltinCommandType::new(InstructionSuperType::Copy,2,Box::new(|x| Ok(Box::new(CopyCommand(x[0],x[1]))))))?;
    set.push("append",7,BuiltinCommandType::new(InstructionSuperType::Append,2,Box::new(|x| Ok(Box::new(AppendCommand(x[0],x[1]))))))?;
    set.push("length",8,BuiltinCommandType::new(InstructionSuperType::Length,2,Box::new(|x| Ok(Box::new(LengthCommand(x[0],x[1]))))))?;
    set.push("add",9,BuiltinCommandType::new(InstructionSuperType::Add,2,Box::new(|x| Ok(Box::new(AddCommand(x[0],x[1]))))))?;
    set.push("numeq",10,BuiltinCommandType::new(InstructionSuperType::NumEq,3,Box::new(|x| Ok(Box::new(NumEqCommand(x[0],x[1],x[2]))))))?;
    set.push("filter",11,BuiltinCommandType::new(InstructionSuperType::Filter,3,Box::new(|x| Ok(Box::new(FilterCommand(x[0],x[1],x[2]))))))?;
    set.push("run",12,BuiltinCommandType::new(InstructionSuperType::Run,3,Box::new(|x| Ok(Box::new(RunCommand(x[0],x[1],x[2]))))))?;
    set.push("seqfilter",13,BuiltinCommandType::new(InstructionSuperType::SeqFilter,4,Box::new(|x| Ok(Box::new(SeqFilterCommand(x[0],x[1],x[2],x[3]))))))?;
    set.push("seqat",14,BuiltinCommandType::new(InstructionSuperType::SeqAt,2,Box::new(|x| Ok(Box::new(SeqAtCommand(x[0],x[1]))))))?;
    set.push("at",15,BuiltinCommandType::new(InstructionSuperType::At,2,Box::new(|x| Ok(Box::new(AtCommand(x[0],x[1]))))))?;
    set.push("refilter",16,BuiltinCommandType::new(InstructionSuperType::ReFilter,3,Box::new(|x| Ok(Box::new(ReFilterCommand(x[0],x[1],x[2]))))))?;
    Ok(set)
}
