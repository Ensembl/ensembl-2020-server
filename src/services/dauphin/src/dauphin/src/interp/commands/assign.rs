use std::rc::Rc;
use crate::model::{ Register, VectorRegisters, ComplexRegisters };
use crate::interp::values::registers::RegisterFile;
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
fn assign_filtered(context: &mut InterpContext, complexes: &Vec<ComplexRegisters>, regs: &Vec<Register>) -> Result<(),String> {
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
    let mut left_start = 1;
    let mut right_start = (regs.len()+1)/2;
    /* get lengths while we can be gurarnteed a shared borrow */
    let mut prep = vec![];
    let assignments1 = complexes[1].iter().map(|x| x.1.clone()).collect::<Vec<_>>();
    let assignments2 = complexes[2].iter().map(|x| x.1.clone()).collect::<Vec<_>>();
    for a_idx in 0..assignments1.len() {
        let a_left = &assignments1[a_idx];
        let a_right = &assignments2[a_idx];
        let depth = a_left.depth();
        let mut do_filter : Option<&Vec<usize>> = Some(filter);
        for level in (0..depth).rev() {
            /* how long are the lower registers? */
            let left_lower = left_start + if level > 0 { a_left.level_offset(level-1).unwrap() } else { a_left.data() };
            let left_lower_len = registers.len(&regs[left_lower])?;
            let right_lower = right_start + if level > 0 { a_right.level_offset(level-1).unwrap() } else { a_right.data() };
            let right_lower_len = registers.len(&regs[right_lower])?;
            let left_self = left_start + a_left.level_offset(level).unwrap();
            let right_self = right_start + a_right.level_offset(level).unwrap();            
            prep.push((left_self,right_self,do_filter,Some((left_lower_len,right_lower_len)),level == depth-1));
            let left_self = left_start + a_left.level_length(level).unwrap();
            let right_self = right_start + a_right.level_length(level).unwrap();            
            prep.push((left_self,right_self,do_filter,Some((0,0)),level == depth-1));
            do_filter = None;
        }
        let left_self = left_start + a_left.data();
        let right_self = right_start + a_right.data();
        prep.push((left_self,right_self,do_filter,None,depth == 0));
        left_start += a_left.register_count();
        right_start += a_right.register_count();
    }
    /* now do it */
    for (left_self,right_self,our_filter,gait,top) in prep {
        if let Some((ref start,ref stride)) = gait {
            if top {
                assign_reg(registers,regs,left_self,right_self, |left,right| {
                    blit_number(left,&right,our_filter,*start,*stride)
                })?;
            } else {
                assign_reg(registers,regs,left_self,right_self, |mut left,right| {
                    for i in 0..filter.len() {
                        left = blit_number(left,right,None,start+i*stride,0)?;
                    }
                    Ok(left)
                })?;
            }
        } else {
            assign_reg(registers,regs,left_self,right_self, |mut left,right| {
                for _ in 0..filter.len() {
                    left = blit(left,right,our_filter)?;
                }
                Ok(left)
            })?;
        }
    }
    Ok(())
}

fn assign(context: &mut InterpContext, filtered: bool, purposes: &Vec<ComplexRegisters>, regs: &Vec<Register>) -> Result<(),String> {
    if filtered {
        assign_filtered(context,purposes,regs)?;
    } else {
        assign_unfiltered(context,regs)?;
    }
    Ok(())
}

pub struct AssignCommand(pub(crate) bool, pub(crate) Vec<ComplexRegisters>, pub(crate) Vec<Register>);

impl Command for AssignCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        assign(context,self.0,&self.1,&self.2)?;
        Ok(())
    }
}
