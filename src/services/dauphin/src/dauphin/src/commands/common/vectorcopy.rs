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
use crate::interp::{ InterpValue, InterpContext, trial_signature };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use crate::commands::common::polymorphic::arbitrate_type;
use crate::typeinf::{ MemberMode, MemberDataFlow, BaseType };
use crate::model::{ Register, VectorRegisters, Identifier, ComplexPath, ComplexRegisters, RegisterSignature };
use regex::Regex;

pub fn do_call_flat(lib: &str, name: &str, impure: bool, spec: &str) -> Result<InstructionType,()> {
    let mut sigs = RegisterSignature::new();
    let mut flows = vec![];
    for cap in Regex::new(r"([RFL])(\d+)(i?o?)([bys]?)").unwrap().captures_iter(spec) {
        let mode = match cap.get(1).ok_or(())?.as_str() {
            "R" => MemberMode::RValue,
            "L" => MemberMode::LValue,
            "F" => MemberMode::FValue,
            _ => Err(())?
        };
        let base = match cap.get(3).ok_or(())?.as_str() {
            "b" => BaseType::BooleanType,
            "y" => BaseType::BytesType,
            "t" => BaseType::StringType,
            _ => BaseType::NumberType
        };
        let depth : usize = cap.get(2).ok_or(())?.as_str().parse::<usize>().map_err(|_| ())?;
        let mut cr = ComplexRegisters::new_empty(mode);
        cr.add(ComplexPath::new_empty(),VectorRegisters::new(depth,base),&vec![depth]);
        sigs.add(cr);
        let flow_s = cap.get(3).ok_or(())?.as_str();
        flows.push(if flow_s.contains("o") {
            if flow_s.contains("i") {
                MemberDataFlow::InOut
            } else {
                MemberDataFlow::Out
            }
        } else {
            MemberDataFlow::In
        });
    }
    Ok(InstructionType::Call(Identifier::new(lib,name),impure,sigs,flows))
}

pub fn call_flat(lib: &str, name: &str, pure_: bool, spec: &str) -> Result<InstructionType,String> {
    do_call_flat(lib,name,pure_,spec).map_err(|_| format!("could not call_flat"))
}

fn update_poly<T>(dst: &mut Vec<T>, src: &Vec<T>, filter: &[usize]) where T: Clone {
    let mut target = vec![];
    while target.len() < filter.len() {
        target.append(&mut src.to_vec());
    }
    let mut value_it = target.drain(..);
    for index in filter.iter() {
        dst[*index] = value_it.next().unwrap();
    }
}

pub fn vector_update_poly(dst: InterpValue, src: &Rc<InterpValue>, filter_val: &[usize]) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&dst,src,true) {
        Ok(polymorphic!(dst,[src],natural,(|d,s| {
            update_poly(d,s,filter_val)
        })))
    } else {
        Ok(dst)
    }
}

pub fn append_data(dst: InterpValue, src: &Rc<InterpValue>, copies: usize) -> Result<(InterpValue,usize),String> {
    let offset = src.len();
    if let Some(natural) = arbitrate_type(&dst,src,false) {
        Ok((polymorphic!(dst,[src],natural,(|d: &mut Vec<_>, s: &[_]| {
            for _ in 0..copies {
                d.append(&mut s.to_vec());
            }
        })),offset))
    } else {
        Ok((dst,offset))
    }
}

pub fn vector_append_offsets(dst: &VectorRegisters, src: &VectorRegisters, 
                             start: &Register, stride: &Register, copies: &Register, regs: &[Register], level: usize) -> Result<Instruction,String> {
    let itype = call_flat("std","_vector_append_indexes",false,"L0ioR0R0R0R0")?;
    Ok(Instruction::new(itype,vec![regs[dst.offset_pos(level)?].clone(),regs[src.offset_pos(level)?].clone(),
                                         start.clone(),stride.clone(),copies.clone()]))
}

pub fn vector_append_lengths(dst: &VectorRegisters, src: &VectorRegisters, 
                            zero: &Register, copies: &Register, regs: &[Register], level: usize) -> Result<Instruction,String> {
    let itype = call_flat("std","_vector_append_indexes",false,"L0ioR0R0R0R0")?;
    Ok(Instruction::new(itype,vec![regs[dst.length_pos(level)?].clone(),regs[src.length_pos(level)?].clone(),
                                        zero.clone(),zero.clone(),copies.clone()]))

}
pub fn vector_update_offsets(dst: &VectorRegisters, src: &VectorRegisters, 
                            start: &Register, stride: &Register, filter: &Register, regs: &[Register], level: usize) -> Result<Instruction,String> {
    let itype = call_flat("std","_vector_update_indexes",false,"L0ioR0R0R0R0")?;
    Ok(Instruction::new(itype,vec![regs[dst.offset_pos(level)?].clone(),regs[src.offset_pos(level)?.clone()],filter.clone(),start.clone(),stride.clone()]))
}

pub fn vector_update_lengths(dst: &VectorRegisters, src: &VectorRegisters, 
                            zero: &Register, filter: &Register, regs: &[Register], level: usize) -> Result<Instruction,String> {
    let itype = call_flat("std","_vector_update_indexes",false,"L0ioR0R0R0R0")?;
    Ok(Instruction::new(itype,vec![regs[dst.length_pos(level)?].clone(),regs[src.length_pos(level)?.clone()],filter.clone(),zero.clone(),zero.clone()]))
}

pub fn vector_append(dst: &VectorRegisters, src: &VectorRegisters, copies: &Register, regs: &[Register]) -> Result<Instruction,String> {
    let itype = call_flat("std","_vector_append",false,"L0ioR0R0")?;
    Ok(Instruction::new(itype,vec![regs[dst.data_pos()].clone(),regs[src.data_pos()].clone(),copies.clone()]))
}

pub fn vector_copy(dst: &VectorRegisters, src: &VectorRegisters, filter: &Register, regs: &[Register]) -> Result<Instruction,String> {
    let itype = call_flat("std","_vector_copy_shallow",false,"L1ioR0R0")?;
    Ok(Instruction::new(itype,vec![regs[dst.data_pos()],regs[src.data_pos()],filter.clone()]))
}

pub fn vector_push_instrs(context: &mut PreImageContext, dst: &VectorRegisters, src: &VectorRegisters, copies: &Register, regs: &[Register]) -> Result<Vec<Instruction>,String> {
    let mut out = vec![];
    let depth = dst.depth();
    /* intermediate levels */
    let zero = context.new_register();
    out.push(Instruction::new(InstructionType::Const(vec![0]),vec![zero]));
    for level in (0..(depth-1)).rev() {
        let start = context.new_register();
        let off = if level > 0 { dst.offset_pos(level-1)? } else { dst.data_pos() };
        out.push(Instruction::new(InstructionType::Length,vec![start,regs[off]]));
        let stride = context.new_register();
        let off = if level > 0 { src.offset_pos(level-1)? } else { src.data_pos() };
        out.push(Instruction::new(InstructionType::Length,vec![stride,regs[off]]));
        out.push(vector_append_offsets(dst,src,&start,&stride,copies,regs,level)?);
        out.push(vector_append_lengths(dst,src,&zero,copies,regs,level)?);
    }
    /* bottom-level */
    out.push(vector_append(dst,src,copies,&regs)?);
    Ok(out)
}

pub fn vector_register_copy_instrs(dst: &VectorRegisters, src: &VectorRegisters, regs: &[Register]) -> Result<Vec<Instruction>,String> {
    let mut out = vec![];
    for level in 0..dst.depth() {
        out.push(Instruction::new(InstructionType::Copy,vec![regs[dst.offset_pos(level)?].clone(),regs[src.offset_pos(level)?].clone()]));
        out.push(Instruction::new(InstructionType::Copy,vec![regs[dst.length_pos(level)?].clone(),regs[src.length_pos(level)?].clone()]));
    }
    out.push(Instruction::new(InstructionType::Copy,vec![regs[dst.data_pos()].clone(),regs[src.data_pos()].clone()]));
    Ok(out)
}
