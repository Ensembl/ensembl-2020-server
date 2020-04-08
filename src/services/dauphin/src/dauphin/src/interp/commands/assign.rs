use std::rc::Rc;
use std::collections::HashMap;
use crate::model::{ LinearPath, Register, RegisterPurpose };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::super::context::{InterpContext };
use crate::interp::{ InterpValue, InterpNatural };
use super::super::command::Command;

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


/// XXX ban multi-Lvalue
fn assign_filtered(context: &mut InterpContext, types: &Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>, regs: &Vec<Register>) -> Result<(),String> {
    let registers = context.registers();
    let len = (regs.len()-1)/2;
    let filter_reg = registers.get_indexes(&regs[0])?;
    let filter = &filter_reg;
    let mut right_all_reg = regs[len+1..].iter().map(|x| registers.get(x).clone()).collect::<Vec<_>>();
    let mut right_all = vec![];
    for r in &mut right_all_reg {
        right_all.push(r.borrow().get_shared()?);
    }
    let mut left_all_reg = regs[1..len+1].iter().map(|x| registers.get(x).clone()).collect::<Vec<_>>();
    let mut left_all = vec![];
    for r in &mut left_all_reg {
        left_all.push(r.borrow().get_shared()?);
    }    
    let left_purposes = &types[1].1;
    let right_purposes = &types[2].1;
    /* get current lengths (to calculate offsets) */
    let mut left_len = HashMap::new();
    let mut right_len = HashMap::new();
    for (j,purpose) in right_purposes.iter().enumerate() {
        left_len.insert(purpose.get_linear(),left_all[j].len());
        right_len.insert(purpose.get_linear(),right_all[j].len());
    }
    for (j,purpose) in left_purposes.iter().enumerate() {
        let mut left = left_all_reg[j].borrow_mut().get_exclusive()?;
        let right = &right_all[j];
        let initial_offset = purpose.get_linear().references()
            .and_then(|p| left_len.get(&p).cloned())
            .unwrap_or(0);
        let copy_offset = purpose.get_linear().references()
            .and_then(|p| right_len.get(&p).cloned())
            .unwrap_or(0);
        if purpose.is_top() {
            match purpose.get_linear() {
                LinearPath::Offset(_) => {
                    left = blit_number(left,right,Some(&filter),initial_offset,copy_offset)?;
                },
                LinearPath::Length(_) => {
                    left = blit_number(left,right,Some(&filter),0,0)?;
                },
                LinearPath::Data | LinearPath::Selector => {
                    left = blit(left,right,Some(&filter))?;
                }
            }
        } else {
            match purpose.get_linear() {
                LinearPath::Offset(_) | LinearPath::Length(_) => {
                    for i in 0..filter.len() {
                        left = blit_number(left,right,None,initial_offset+i*copy_offset,0)?;
                    }
                },
                LinearPath::Data | LinearPath::Selector => {
                    for _ in 0..filter.len() {
                        left = blit(left,right,None)?;
                    }
                }
            }
        }
        registers.write(&regs[j+1],left);
    }
    Ok(())
}

fn assign(context: &mut InterpContext, types: &Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>, regs: &Vec<Register>) -> Result<(),String> {
    if types[0].0 == MemberMode::LValue {
        assign_unfiltered(context,regs)?;
    } else {
        assign_filtered(context,types,regs)?;
    }
    Ok(())
}

pub struct AssignCommand(pub(crate) Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>,pub(crate) Vec<Register>);

impl Command for AssignCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        assign(context,&self.0,&self.1)?;
        Ok(())
    }
}
