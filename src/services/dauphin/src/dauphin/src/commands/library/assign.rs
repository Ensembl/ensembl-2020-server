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
use crate::model::{ Register, RegisterSignature };
use crate::interp::{ InterpValue, InterpNatural };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, RegisterFile };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_array, cbor_bool };
use crate::typeinf::MemberMode;

fn blit_typed<T>(dst: &mut Vec<T>, src: &Vec<T>, filter: Option<&Vec<usize>>) where T: Clone {
    if let Some(filter) = filter {
        let src_len = src.len();
        for (i,filter_pos) in filter.iter().enumerate() {
            dst[*filter_pos] = src[i%src_len].clone();
        }
    } else {
        let mut new_values : Vec<T> = src.to_vec();
        dst.append(&mut new_values);
    }
}

fn blit_expanded_typed<T>(dst: &mut Vec<T>, src: &Vec<T>, filter: &Vec<bool>) where T: Clone {
    let filter_len = filter.len();
    for (i,value) in src.iter().enumerate() {
        if filter[i%filter_len] {
            dst.push(value.clone());
        }
    }
}

fn blit_runs_typed<T>(dst: &mut Vec<T>, src: &Vec<T>, starts: &Vec<usize>, lens: &Vec<usize>) where T: Clone {
    let starts_len = starts.len();
    let lens_len = lens.len();
    let src_len = src.len();
    for i in 0..starts_len {
        for j in 0..lens[i%lens_len] {
            dst.push(src[(starts[i]+j)%src_len].clone());
        }
    }
}

pub fn coerce_to(dst: &InterpValue, src: &Rc<InterpValue>, prefer_dst: bool) -> Option<InterpNatural> {
    let src_natural = src.get_natural();
    let dst_natural = dst.get_natural();
    if let InterpNatural::Empty = src_natural { return None; }
    Some(if let InterpNatural::Empty = dst_natural {
        src_natural
    } else {
        if prefer_dst { dst_natural } else { src_natural }
    })
}

// If only there were higher-order type bounds in where clauses!
macro_rules! run_typed {
    ($dst:ident,$src:ident,$natural:expr,$func:tt) => {
        match $natural {
            InterpNatural::Empty => { $dst }, /* impossible due to ifs above */
            InterpNatural::Numbers => { let s = $src.to_rc_numbers()?.0; let mut d = $dst.to_numbers()?; $func(&mut d,&s); InterpValue::Numbers(d) },
            InterpNatural::Indexes => { let s = $src.to_rc_indexes()?.0; let mut d = $dst.to_indexes()?; $func(&mut d,&s); InterpValue::Indexes(d) },
            InterpNatural::Boolean => { let s = $src.to_rc_boolean()?.0; let mut d = $dst.to_boolean()?; $func(&mut d,&s); InterpValue::Boolean(d) },
            InterpNatural::Strings => { let s = $src.to_rc_strings()?.0; let mut d = $dst.to_strings()?; $func(&mut d,&s); InterpValue::Strings(d) },
            InterpNatural::Bytes => { let s = $src.to_rc_bytes()?.0; let mut d = $dst.to_bytes()?; $func(&mut d,&s); InterpValue::Bytes(d) },
        }
    };
}

pub fn blit(dst: InterpValue, src: &Rc<InterpValue>, filter_val: Option<&Vec<usize>>) -> Result<InterpValue,String> {
    if let Some(natural) = coerce_to(&dst,src,filter_val.is_some()) {
        Ok(run_typed!(dst,src,natural,(|d,s| {
            blit_typed(d,s,filter_val)
        })))
    } else {
        Ok(dst)
    }
}

pub fn blit_expanded(dst: InterpValue, src: &Rc<InterpValue>, filter_val: &Vec<bool>) -> Result<InterpValue,String> {
    if let Some(natural) = coerce_to(&dst,src,true) {
        Ok(run_typed!(dst,src,natural,(|d,s| {
            blit_expanded_typed(d,s,filter_val)
        })))
    } else {
        Ok(dst)
    }
}

pub fn blit_runs(dst: InterpValue, src: &Rc<InterpValue>, starts: &Vec<usize>, lens: &Vec<usize>) -> Result<InterpValue,String> {
    if let Some(natural) = coerce_to(&dst,src,true) {
        Ok(run_typed!(dst,src,natural,(|d,s| {
            blit_runs_typed(d,s,starts,lens)
        })))
    } else {
        Ok(dst)
    }
}

fn blit_number(dst: InterpValue, src: &Rc<InterpValue>, filter: Option<&Vec<usize>>, offset: usize, stride: usize) -> Result<InterpValue,String> {
    let srcv = src.to_rc_indexes()?.0;
    let mut dstv = dst.to_indexes()?;
    let src = &srcv;
    if let Some(filter) = filter {
        let src_len = src.len();
        for (i,filter_pos) in filter.iter().enumerate() {
            dstv[*filter_pos] = src[i%src_len] + offset + (i*stride);
        }
    } else {
        let mut new_values = src.iter().map(|x| *x+offset).collect();
        dstv.append(&mut new_values);
    }
    Ok(InterpValue::Indexes(dstv))
}

fn assign_unfiltered(context: &mut InterpContext, regs: &Vec<Register>) -> Result<(),String> {
    let registers = context.registers();
    let n = regs.len()/2;
    for i in 0..n {
        registers.copy(&regs[i],&regs[i+n])?;
    }
    Ok(())
}

fn assign_reg<T>(registers: &mut RegisterFile, regs: &[Register], left_idx: usize, right_idx: usize, cb: T) -> Result<(),String> 
        where T: Fn(InterpValue,&Rc<InterpValue>) -> Result<InterpValue,String> {
    let right = registers.get(&regs[right_idx]);
    let right = right.borrow().get_shared()?;
    let left = registers.get(&regs[left_idx]);
    let left = left.borrow_mut().get_exclusive()?;
    let left = cb(left,&right)?;
    registers.write(&regs[left_idx],left);
    Ok(())
}

/// XXX ban multi-Lvalue
fn assign_filtered(context: &mut InterpContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    let registers = context.registers();
    let filter_reg = registers.get_indexes(&regs[0])?;
    let filter = &filter_reg;
    /* get lengths while we can be gurarnteed a shared borrow */
    let assignments1 = sig[1].iter().map(|x| x.1.clone()).collect::<Vec<_>>();
    let assignments2 = sig[2].iter().map(|x| x.1.clone()).collect::<Vec<_>>();
    let mut lengths = vec![];
    for a_idx in 0..assignments1.len() {
        let a_left = &assignments1[a_idx];
        let a_right = &assignments2[a_idx];
        let depth = a_left.depth();
        let mut level_lengths = vec![];
        for level in 0..depth {
            /* how long are the lower registers? */
            let left_lower_len = registers.len(&regs[a_left.lower_pos(level)])?;
            let right_lower_len = registers.len(&regs[a_right.lower_pos(level)])?;
            level_lengths.push((left_lower_len,right_lower_len));
        }
        lengths.push(level_lengths);
    }
    /* now do it */
    let mut our_filter : Option<&Vec<usize>> = Some(filter);
    for a_idx in 0..assignments1.len() {
        let a_left = &assignments1[a_idx];
        let a_right = &assignments2[a_idx];
        let depth = a_left.depth();
        for level in (0..depth).rev() {
            let (start,stride) = &lengths[a_idx][level];
            if level == a_left.depth()-1 {
                assign_reg(registers,regs,a_left.offset_pos(level)?,a_right.offset_pos(level)?, |left,right| {
                    blit_number(left,&right,our_filter,*start,*stride)
                })?;
                assign_reg(registers,regs,a_left.length_pos(level)?,a_right.length_pos(level)?, |left,right| {
                    blit_number(left,&right,our_filter,0,0)
                })?;
            } else {
                assign_reg(registers,regs,a_left.offset_pos(level)?,a_right.offset_pos(level)?, |mut left,right| {
                    for i in 0..filter.len() {
                        left = blit_number(left,right,None,start+i*stride,0)?;
                    }
                    Ok(left)
                })?;
                assign_reg(registers,regs,a_left.length_pos(level)?,a_right.length_pos(level)?, |mut left,right| {
                    for _ in 0..filter.len() {
                        left = blit_number(left,right,None,0,0)?;
                    }
                    Ok(left)
                })?;
            }
            our_filter = None;
        }
        assign_reg(registers,regs,a_left.data_pos(),a_right.data_pos(), |mut left,right| {
            for _ in 0..filter.len() {
                left = blit(left,right,our_filter)?;
            }
            Ok(left)
        })?;
    }
    Ok(())
}

fn assign(context: &mut InterpContext, filtered: bool, purposes: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    if filtered {
        assign_filtered(context,purposes,regs)?;
    } else {
        assign_unfiltered(context,regs)?;
    }
    Ok(())
}

pub struct AssignCommandType();

impl CommandType for AssignCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command("assign".to_string())
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(AssignCommand(sig[0].get_mode() != MemberMode::LValue,sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[2],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(&value[1],false)?;
        Ok(Box::new(AssignCommand(cbor_bool(&value[0])?,sig,regs)))
    }
}

pub struct AssignCommand(pub(crate) bool, pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for AssignCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        assign(context,self.0,&self.1,&self.2)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let regs = CborValue::Array(self.2.iter().map(|x| x.serialize()).collect());
        Ok(vec![CborValue::Bool(self.0),self.1.serialize(false)?,regs])
    }
}

pub(super) fn library_assign_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("assign",9,AssignCommandType())?;
    Ok(())
}