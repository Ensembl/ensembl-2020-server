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
use crate::command::{ CommandDeserializer, CommandSetId, InterpCommand, InterpLibRegister };
use crate::runtime::{ Register, InterpValue, InterpContext };
use crate::types::arbitrate_type;
use serde_cbor::Value as CborValue;
use super::consts::{ const_commands_interp };

pub struct NilDeserializer();

impl CommandDeserializer for NilDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((5,1))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(NilInterpCommand(Register::deserialize(value[0])?)))
    }
}

pub struct NilInterpCommand(Register);

impl InterpCommand for NilInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Empty);
        Ok(())
    }
}

pub struct CopyDeserializer();

impl CommandDeserializer for CopyDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((6,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(CopyInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?)))
    }
}

pub struct CopyInterpCommand(Register,Register);

impl InterpCommand for CopyInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().copy(&self.0,&self.1)?;
        Ok(())
    }
}

pub struct AppendDeserializer();

impl CommandDeserializer for AppendDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((7,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(AppendInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?)))
    }
}

pub struct AppendInterpCommand(Register,Register);

impl InterpCommand for AppendInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let src = registers.get(&self.1).borrow().get_shared()?;
        let dstr = registers.get(&self.0);
        let dst = dstr.borrow_mut().get_exclusive()?;
        registers.write(&self.0,append(dst,&src)?);
        Ok(())
    }
}

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

pub struct LengthDeserializer();

impl CommandDeserializer for LengthDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((8,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(LengthInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?)))
    }
}

pub struct LengthInterpCommand(Register,Register);

impl InterpCommand for LengthInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let len = registers.get(&self.1).borrow().get_shared()?.len();
        registers.write(&self.0,InterpValue::Indexes(vec![len]));
        Ok(())
    }
}

pub struct AddDeserializer();

impl CommandDeserializer for AddDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((9,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(AddInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?)))
    }
}

pub struct AddInterpCommand(Register,Register);

impl InterpCommand for AddInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = registers.take_indexes(&self.0)?;
        let src_len = (&src).len();
        for i in 0..dst.len() {
            dst[i] += src[i%src_len];
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }
}

pub struct ReFilterDeserializer();

impl CommandDeserializer for ReFilterDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((16,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(ReFilterInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,Register::deserialize(value[2])?)))
    }
}

pub struct ReFilterInterpCommand(Register,Register,Register);

impl InterpCommand for ReFilterInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let src : &[usize] = &registers.get_indexes(&self.1)?;
        let indexes : &[usize] = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        for x in indexes.iter() {
            dst.push(src[*x]);
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }
}

pub struct NumEqDeserializer();

impl CommandDeserializer for NumEqDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((10,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(NumEqInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,Register::deserialize(value[2])?)))
    }
}    

pub struct NumEqInterpCommand(Register,Register,Register);

impl InterpCommand for NumEqInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
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

pub struct FilterDeserializer();

impl CommandDeserializer for FilterDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((11,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(FilterInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,Register::deserialize(value[2])?)))
    }
}    

pub struct FilterInterpCommand(Register,Register,Register);

impl InterpCommand for FilterInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let filter_val = registers.get_boolean(&self.2)?;
        let src = registers.get(&self.1);
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,filter(&src,&filter_val)?);
        Ok(())
    }
}

pub struct RunDeserializer();

impl CommandDeserializer for RunDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((12,4))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(RunInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,
                                     Register::deserialize(value[2])?,Register::deserialize(value[3])?)))
    }
}    

pub struct RunInterpCommand(Register,Register,Register,Register);

impl InterpCommand for RunInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let start = &registers.get_indexes(&self.1)?;
        let len = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        let startlen = start.len();
        let lenlen = len.len();
        if lenlen == 0 {
            Err(format!("zero length run in register {:?}\n",self.2))?
        }
        for i in 0..startlen {
            for j in 0..len[i%lenlen] {
                dst.push(start[i]+j);
            }
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }
}

pub struct AtDeserializer();

impl CommandDeserializer for AtDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((15,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(AtInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?)))
    }
}

pub struct AtInterpCommand(Register,Register);

impl InterpCommand for AtInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = vec![];
        for i in 0..src.len() {
            dst.push(i);
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
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

pub struct SeqFilterDeserializer();

impl CommandDeserializer for SeqFilterDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((13,4))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(SeqFilterInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,
                                           Register::deserialize(value[2])?,Register::deserialize(value[3])?)))
    }
}    

pub struct SeqFilterInterpCommand(Register,Register,Register,Register);

impl InterpCommand for SeqFilterInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let src = registers.get(&self.1);
        let start = registers.get_indexes(&self.2)?;
        let len = registers.get_indexes(&self.3)?;
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,seq_filter(&src,&start,&len)?);
        Ok(())
    }
}

pub struct SeqAtDeserializer();

impl CommandDeserializer for SeqAtDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((14,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(SeqAtInterpCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,Register::deserialize(value[2])?)))
    }
}

pub struct SeqAtInterpCommand(Register,Register,Register);

impl InterpCommand for SeqAtInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
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
}

pub struct PauseDeserializer();

impl CommandDeserializer for PauseDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((18,0))) }
    fn deserialize(&self, _opcode: u32, _value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(PauseInterpCommand()))
    }
}

pub struct PauseInterpCommand();

impl InterpCommand for PauseInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.do_pause();
        Ok(())
    }
}

pub fn make_core_interp() -> Result<InterpLibRegister,String> {
    let set_id = CommandSetId::new("core",(0,0),0x6131BA5737E6EAE0);
    let mut set = InterpLibRegister::new(&set_id);
    const_commands_interp(&mut set)?;
    set.push(NilDeserializer());
    set.push(CopyDeserializer());
    set.push(AppendDeserializer());
    set.push(LengthDeserializer());
    set.push(AddDeserializer());
    set.push(NumEqDeserializer());
    set.push(FilterDeserializer());
    set.push(RunDeserializer());
    set.push(SeqFilterDeserializer());
    set.push(SeqAtDeserializer());
    set.push(AtDeserializer());
    set.push(ReFilterDeserializer());
    set.push(PauseDeserializer());
    Ok(set)
}