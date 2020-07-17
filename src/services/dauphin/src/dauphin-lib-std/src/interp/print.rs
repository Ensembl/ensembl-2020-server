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
use dauphin_interp::command::{ InterpCommand, InterpLibRegister, CommandDeserializer };
use dauphin_interp::types::{ SharedVec, RegisterSignature, XStructure, RegisterVectorSource, VectorRegisters, to_xstructure };
use dauphin_interp::runtime::{ InterpContext, InterpValue, InterpNatural, Register, RegisterFile };
use dauphin_interp::util::cbor::cbor_array;
use crate::stream::Stream;
use serde_cbor::Value as CborValue;

// XXX dedup
pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
    let p = context.payload("std","stream")?;
    Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
}

fn print_simple(sv: &SharedVec, path: &[usize], first: usize) -> Result<String,String> {
    let (data,offset) = vr_lookup_data(sv,path,first)?;
    Ok(match data.get_natural() {
        InterpNatural::Empty => "".to_string(),
        InterpNatural::Indexes => format!("{}",data.to_rc_indexes()?.0[offset]),
        InterpNatural::Numbers => format!("{}",data.to_rc_numbers()?.0[offset]),
        InterpNatural::Boolean => format!("{}",data.to_rc_boolean()?.0[offset]),
        InterpNatural::Strings => format!("\"{}\"",data.to_rc_strings()?.0[offset]),
        InterpNatural::Bytes => format!("\'{}\'",data.to_rc_bytes()?.0[offset].iter().map(|x| format!("{:02x}",x)).collect::<Vec<_>>().join("")),
    })
}

fn vr_lookup_data(sv: &SharedVec, path: &[usize], first: usize) -> Result<(Rc<InterpValue>,usize),String> {
    let mut position = first;
    for (i,index) in path.iter().enumerate() {
        let offset_val = sv.get_offset(sv.depth()-1-i)?;
        position = offset_val[position] + index;
    }
    Ok((sv.get_data().clone(),position))
}

fn vr_lookup_len(sv: &SharedVec, path: &[usize], first: usize) -> Result<usize,String> {
    let mut position = first;
    for (i,index) in path.iter().enumerate() {
        let offset_val = sv.get_offset(sv.depth()-1-i)?;
        position = offset_val[position] + index;
    }
    let len_val = sv.get_length(sv.depth()-1-path.len())?;
    Ok(len_val[position])
}

fn print(file: &RegisterFile, xs: &XStructure<SharedVec>, regs: &[Register], path: &[usize], first: usize) -> Result<String,String> {
    Ok(match xs {
        XStructure::Vector(xs_inner) => {
            let sv = xs.any();
            let len = vr_lookup_len(&sv,path,first)?;
            let mut out = vec![];
            for i in 0..len {
                let mut new_path = path.to_vec();
                new_path.push(i);
                out.push(print(file,xs_inner,regs,&new_path,first)?);
            }
            format!("[{}]",out.join(", "))
        },
        XStructure::Struct(id,kvs) => {
            let mut subs : Vec<String> = kvs.keys().cloned().collect();
            subs.sort();
            let kvs : Vec<(String,_)> = subs.drain(..).map(|k| (k.clone(),kvs.get(&k).unwrap().clone())).collect();
            let out = kvs.iter().map(|(name,xs_inner)| 
                Ok(format!("{}: {}",name,print(file,xs_inner,regs,path,first)?))
            ).collect::<Result<Vec<_>,String>>()?;
            format!("{} {{ {} }}",id.to_string(),out.join(", "))
        },
        XStructure::Enum(id,order,kvs,disc) => {
            let (data,offset) = vr_lookup_data(&disc.borrow(),path,first)?;
            let disc_val = data.to_rc_indexes()?.0[offset];
            let inner_xs = kvs.get(&order[disc_val]).ok_or_else(|| format!("bad enum"))?;
            format!("{}:{} {}",id,order[disc_val],print(file,inner_xs,regs,path,first)?)
        },
        XStructure::Simple(sv) => print_simple(&sv.borrow(),path,first)?,
    })
}

pub struct PrintInterpCommand(Register);

impl InterpCommand for PrintInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let ss = registers.get_strings(&self.0)?;
        for s in ss.iter() {
            std_stream(context)?.add(s);
        }
        Ok(())
    }
}

pub struct FormatDeserializer();

impl CommandDeserializer for FormatDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((2,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(value[1],true)?;
        Ok(Box::new(FormatInterpCommand(regs,sig)))        
    }
}

pub struct FormatInterpCommand(Vec<Register>,RegisterSignature);

impl InterpCommand for FormatInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let xs = to_xstructure(&self.1[1])?;
        let vs = RegisterVectorSource::new(&self.0);
        let xs2 = xs.derive(&mut (|vr: &VectorRegisters| SharedVec::new(context,&vs,vr)))?;
        let sv = xs2.any();
        let num = if sv.depth() > 0 { sv.get_offset(sv.depth()-1)?.len() } else { sv.get_data().len() };
        let registers = context.registers_mut();
        let mut out = vec![];
        for i in 0..num {
            out.push(print(&registers,&xs2,&self.0,&vec![],i)?);
        }
        registers.write(&self.0[0],InterpValue::Strings(out));
        Ok(())
    }
}

pub struct PrintDeserializer();

impl CommandDeserializer for PrintDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((14,1))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(PrintInterpCommand(Register::deserialize(value[0])?)))
    }
}

pub(super) fn library_print_commands_interp(set: &mut InterpLibRegister) -> Result<(),String> {
    set.push(PrintDeserializer());
    set.push(FormatDeserializer());
    Ok(())
}