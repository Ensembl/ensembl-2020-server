use std::collections::HashMap;
use crate::model::{ LinearPath, Register, RegisterPurpose };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::super::context::{InterpContext };
use super::super::value::{ InterpValueData, InterpNatural, ReadOnlyValues, ReadWriteValues };
use super::super::command::Command;

fn blit_typed<T>(dst: ReadWriteValues<T>, src: ReadOnlyValues<T>, filter: Option<&Vec<usize>>) where T: Clone {
    if let Some(filter) = filter {
        let src = src.borrow();
        let src_len = src.len();
        let mut dst = dst.borrow_mut();
        for (i,filter_pos) in filter.iter().enumerate() {
            dst[*filter_pos] = src[i%src_len].clone();
        }
    } else {
        let mut new_values : Vec<T> = src.borrow().to_vec();
        dst.borrow_mut().append(&mut new_values);
    }
}

fn blit_expanded_typed<T>(dst: ReadWriteValues<T>, src: ReadOnlyValues<T>, filter: &Vec<bool>) where T: Clone {
    let filter_len = filter.len();
    let src = src.borrow();
    let mut dst = dst.borrow_mut();
    for (i,value) in src.iter().enumerate() {
        if filter[i%filter_len] {
            dst.push(value.clone());
        }
    }
}

fn blit_runs_typed<T>(dst: ReadWriteValues<T>, src: ReadOnlyValues<T>, starts: &Vec<usize>, lens: &Vec<usize>) where T: Clone {
    let starts_len = starts.len();
    let lens_len = lens.len();
    let src = src.borrow();
    let src_len = src.len();
    let mut dst = dst.borrow_mut();
    for i in 0..starts_len {
        for j in 0..lens[i%lens_len] {
            dst.push(src[(i+j)%src_len].clone());
        }
    }
}

pub fn coerce_to(dst: &InterpValueData, src: &InterpValueData, prefer_dst: bool) -> Option<InterpNatural> {
    let src_natural = src.get_natural();
    let dst_natural = dst.get_natural();
    if let InterpNatural::Empty = src_natural { return None; }
    Some(if let InterpNatural::Empty = dst_natural {
        src_natural
    } else {
        if prefer_dst { dst_natural } else { src_natural }
    })
}

pub fn blit(dst: &mut InterpValueData, src: &InterpValueData, filter_val: Option<&Vec<usize>>) -> Result<(),String> {
    if let Some(natural) = coerce_to(dst,src,filter_val.is_some()) {
        match natural {
            InterpNatural::Empty => {}, /* impossible due to ifs above */
            InterpNatural::Numbers => { let v = src.read_numbers()?; blit_typed(dst.write_numbers()?,v,filter_val); },
            InterpNatural::Indexes => { let v = src.read_indexes()?; blit_typed(dst.write_indexes()?,v,filter_val); },
            InterpNatural::Boolean => { let v = src.read_boolean()?; blit_typed(dst.write_boolean()?,v,filter_val); },
            InterpNatural::Strings => { let v = src.read_strings()?; blit_typed(dst.write_strings()?,v,filter_val); },
            InterpNatural::Bytes => { let v = src.read_bytes()?; blit_typed(dst.write_bytes()?,v,filter_val); },
        }
    }
    Ok(())
}

pub fn blit_expanded(dst: &mut InterpValueData, src: &InterpValueData, filter_val: &Vec<bool>) -> Result<(),String> {
    if let Some(natural) = coerce_to(dst,src,true) {
        match natural {
            InterpNatural::Empty => {}, /* impossible due to ifs above */
            InterpNatural::Numbers => { let v = src.read_numbers()?; blit_expanded_typed(dst.write_numbers()?,v,filter_val); },
            InterpNatural::Indexes => { let v = src.read_indexes()?; blit_expanded_typed(dst.write_indexes()?,v,filter_val); },
            InterpNatural::Boolean => { let v = src.read_boolean()?; blit_expanded_typed(dst.write_boolean()?,v,filter_val); },
            InterpNatural::Strings => { let v = src.read_strings()?; blit_expanded_typed(dst.write_strings()?,v,filter_val); },
            InterpNatural::Bytes => { let v = src.read_bytes()?; blit_expanded_typed(dst.write_bytes()?,v,filter_val); },
        }
    }
    Ok(())
}

pub fn blit_runs(dst: &mut InterpValueData, src: &InterpValueData, starts: &Vec<usize>, lens: &Vec<usize>) -> Result<(),String> {
    if let Some(natural) = coerce_to(dst,src,true) {
        match natural {
            InterpNatural::Empty => {}, /* impossible due to ifs above */
            InterpNatural::Numbers => { let v = src.read_numbers()?; blit_runs_typed(dst.write_numbers()?,v,starts,lens); },
            InterpNatural::Indexes => { let v = src.read_indexes()?; blit_runs_typed(dst.write_indexes()?,v,starts,lens); },
            InterpNatural::Boolean => { let v = src.read_boolean()?; blit_runs_typed(dst.write_boolean()?,v,starts,lens); },
            InterpNatural::Strings => { let v = src.read_strings()?; blit_runs_typed(dst.write_strings()?,v,starts,lens); },
            InterpNatural::Bytes => { let v = src.read_bytes()?; blit_runs_typed(dst.write_bytes()?,v,starts,lens); },
        }
    }
    Ok(())
}

fn blit_number(dst: &mut InterpValueData, src: &InterpValueData, filter: Option<&Vec<usize>>, offset: usize, stride: usize) -> Result<(),String> {
    let srcv = src.read_indexes()?;
    let dstv = dst.write_indexes()?;
    let src = srcv.borrow();
    let mut dst = dstv.borrow_mut();
    if let Some(filter) = filter {
        let src_len = src.len();
        for (i,filter_pos) in filter.iter().enumerate() {
            dst[*filter_pos] = src[i%src_len] + offset + (i*stride);
        }
    } else {
        let mut new_values = src.iter().map(|x| *x+offset).collect();
        dst.append(&mut new_values);
    }
    Ok(())
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
    let filter_reg = registers.read_indexes(&regs[0])?;
    let filter = filter_reg.borrow();
    let mut right_all_reg = regs[len+1..].iter().map(|x| registers.get(x).clone()).collect::<Vec<_>>();
    let mut right_all = vec![];
    for r in &mut right_all_reg {
        right_all.push(r.read()?);
    }
    let mut left_all_reg = regs[1..len+1].iter().map(|x| registers.get(x).clone()).collect::<Vec<_>>();
    let mut left_all = vec![];
    for r in &mut left_all_reg {
        let v = r.write();
        left_all.push(v);
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
    // XXX overlapping writes
    for (j,purpose) in left_purposes.iter().enumerate() {
        let left = &mut left_all[j];
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
                    blit_number(left,right,Some(&filter),initial_offset,copy_offset)?;
                },
                LinearPath::Length(_) => {
                    blit_number(left,right,Some(&filter),0,0)?;
                },
                LinearPath::Data => {
                    blit(left,right,Some(&filter))?;
                }
            }
        } else {
            match purpose.get_linear() {
                LinearPath::Offset(_) | LinearPath::Length(_) => {
                    for i in 0..filter.len() {
                        blit_number(left,right,Some(&filter),initial_offset+i*copy_offset,0)?;
                    }
                },
                LinearPath::Data => {
                    for _ in 0..filter.len() {
                        blit(left,right,None)?;
                    }
                }
            }
        }
    }
    drop(left_all);
    for r in left_all_reg.drain(..) {
        registers.add_commit(r);
    }
    Ok(())
}

fn assign(context: &mut InterpContext, types: &Vec<(MemberMode,Vec<RegisterPurpose>,MemberDataFlow)>, regs: &Vec<Register>) -> Result<(),String> {
    if types[0].0 == MemberMode::LValue {
        assign_unfiltered(context,regs);
    } else {
        assign_filtered(context,types,regs);
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
