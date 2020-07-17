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

use std::fmt::Debug;
use std::rc::Rc;
use dauphin_interp::command::{ CommandDeserializer, InterpCommand, InterpLibRegister };
use dauphin_interp::runtime::{ Register };
use dauphin_interp::types::{
    SharedVec, RegisterVectorSource, VectorRegisters,
    arbitrate_type
};
use dauphin_interp::runtime::{ InterpContext, InterpValue };
use dauphin_interp::util::cbor::{ cbor_array };
use serde_cbor::Value as CborValue;

fn compare_work<T>(a: &SharedVec, a_off: (usize,usize), a_data: &[T], b: &SharedVec, b_off: (usize,usize), b_data: &[T], level: usize) -> Result<bool,String>
        where T: PartialEq {
    if a_off.1 != b_off.1 { return Ok(false); }
    if level > 0 {
        /* index with index below */
        let lower_a_off = a.get_offset(level-1)?;
        let lower_a_len = a.get_length(level-1)?;
        let lower_b_off = b.get_offset(level-1)?;
        let lower_b_len = b.get_length(level-1)?;
        for i in 0..a_off.1 {
            if !compare_work(a,(lower_a_off[a_off.0+i],lower_a_len[a_off.0+i]),a_data,
                                b,(lower_b_off[b_off.0+i],lower_b_len[b_off.0+i]),b_data,
                                level-1)? {
                return Ok(false);
            }
        }
    } else {
        /* index with data below */
        for i in 0..a_off.1 {
            if a_data[a_off.0+i] != b_data[b_off.0+i] {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn compare_indexed<T>(a: &SharedVec, b: &SharedVec, a_data: &[T], b_data: &[T]) -> Result<Vec<bool>,String> where T: PartialEq + Debug {
    let top_a_off = a.get_offset(a.depth()-1)?;
    let top_a_len = a.get_length(a.depth()-1)?;
    let top_b_off = b.get_offset(b.depth()-1)?;
    let top_b_len = b.get_length(b.depth()-1)?;
    let b_len = top_b_off.len();
    let mut out = vec![];
    for i in 0..top_a_off.len() {
        out.push(compare_work(a,(top_a_off[i],top_a_len[i]),a_data,
                              b,(top_b_off[i%b_len],top_b_len[i%b_len]),b_data,
                              a.depth()-1)?);
    }
    Ok(out)
}

fn compare_data<T>(a: &[T], b: &[T]) -> Vec<bool> where T: PartialEq {
    let b_len = b.len();
    a.iter().enumerate().map(|(i,av)| av == &b[i%b_len]).collect()
}

pub fn compare(a: &SharedVec, b: &SharedVec) -> Result<Vec<bool>,String> {
    if a.depth() != b.depth() {
        return Err(format!("unequal types in eq"));
    }
    let a_data = a.get_data();
    let b_data = b.get_data();
    if let Some(natural) = arbitrate_type(&a_data,&b_data,true) {
        Ok(dauphin_interp::polymorphic!([&a_data,&b_data],natural,(|d,s| {
            compare_indexed(a,b,d,s)
        })).transpose()?.ok_or_else(|| format!("unexpected empty in eq"))?)
    } else {
        Ok(vec![])
    }
}


pub struct EqCompareDeserializer();

impl CommandDeserializer for EqCompareDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((19,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let regs = cbor_array(&value[2],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let a = VectorRegisters::deserialize(&value[0])?;
        let b = VectorRegisters::deserialize(&value[1])?;
        Ok(Box::new(EqCompareInterpCommand(a,b,regs)))
    }
}

pub struct EqCompareInterpCommand(VectorRegisters,VectorRegisters,Vec<Register>);

impl InterpCommand for EqCompareInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let vs = RegisterVectorSource::new(&self.2);
        let a = SharedVec::new(context,&vs,&self.0)?;
        let b = SharedVec::new(context,&vs,&self.1)?;
        let result = compare(&a,&b)?;
        context.registers_mut().write(&self.2[0],InterpValue::Boolean(result));
        Ok(())
    }
}


pub struct EqShallowDeserializer();

impl CommandDeserializer for EqShallowDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((0,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(EqShallowInterpCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?)))
    }
}

pub struct EqShallowInterpCommand(Register,Register,Register);

impl EqShallowInterpCommand {
    fn compare(&self, a: &Rc<InterpValue>, b: &Rc<InterpValue>) -> Result<Vec<bool>,String> {
        if let Some(natural) = arbitrate_type(a,b,true) {
            let out = dauphin_interp::polymorphic!([a,b],natural,(|d,s| {
                compare_data(d,s)
            })).unwrap_or_else(|| vec![]);
            Ok(out)
        } else {
            Ok(vec![])
        }
    }
}

impl InterpCommand for EqShallowInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let a_data = context.registers().get(&self.1);
        let b_data = context.registers().get(&self.2);
        let a_data = a_data.borrow().get_shared()?;
        let b_data = b_data.borrow().get_shared()?;
        let result = self.compare(&a_data,&b_data)?;
        context.registers_mut().write(&self.0,InterpValue::Boolean(result));
        Ok(())
    }
}


pub struct AllDeserializer();

impl CommandDeserializer for AllDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((20,1))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        Ok(Box::new(AllInterpCommand(regs)))
    }
}

pub struct AllInterpCommand(Vec<Register>);

impl InterpCommand for AllInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let mut out = context.registers().get_boolean(&self.0[1])?.to_vec();
        for reg in &self.0[2..] {
            let more = context.registers().get_boolean(reg)?;
            let mut more = more.iter().cycle();
            for v in out.iter_mut() {
                *v &= more.next().unwrap();
            }
        }
        context.registers_mut().write(&self.0[0],InterpValue::Boolean(out));
        Ok(())
    }
}

pub(super) fn library_eq_command_interp(set: &mut InterpLibRegister) -> Result<(),String> {
    set.push(EqShallowDeserializer());
    set.push(EqCompareDeserializer());
    set.push(AllDeserializer());
    Ok(())
}
