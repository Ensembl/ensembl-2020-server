use super::super::context::{InterpContext };
use super::super::value::{ InterpValueData, InterpNatural, ReadOnlyValues, ReadWriteValues };
use super::super::command::Command;
use crate::model::Register;

fn blit_typed<T>(dst: ReadWriteValues<T>, src: ReadOnlyValues<T>, filter: Option<ReadOnlyValues<usize>>) where T: Clone {
    if let Some(filter) = filter {
        let src = src.borrow();
        let src_len = src.len();
        let mut dst = dst.borrow_mut();
        for (i,filter_pos) in filter.borrow().iter().enumerate() {
            dst[*filter_pos] = src[i%src_len].clone();
        }
    } else {
        let mut new_values : Vec<T> = src.borrow().to_vec();
        dst.borrow_mut().append(&mut new_values);
    }

}

fn blit(context: &mut InterpContext, dst: &Register, src: &Register, filter: Option<&Register>) -> Result<(),String> {
    let registers = context.registers();
    let filter_val = filter.map(|r| registers.read_indexes(r)).transpose()?;
    let src_natural = registers.get_natural(src)?;
    let dst_natural = registers.get_natural(dst)?;
    if let InterpNatural::Empty = src_natural { return Ok(()); }
    let natural = if let InterpNatural::Empty = dst_natural {
        src_natural
    } else {
        match filter {
            Some(_) => registers.get_natural(dst)?,
            None => registers.get_natural(src)?
        }
    };
    match natural {
        InterpNatural::Empty => {}, /* impossible due to ifs above */
        InterpNatural::Numbers => { let v = registers.read_numbers(src)?; blit_typed(registers.modify_numbers(dst)?,v,filter_val); },
        InterpNatural::Indexes => { let v = registers.read_indexes(src)?; blit_typed(registers.modify_indexes(dst)?,v,filter_val); },
        InterpNatural::Boolean => { let v = registers.read_boolean(src)?; blit_typed(registers.modify_boolean(dst)?,v,filter_val); },
        InterpNatural::Strings => { let v = registers.read_strings(src)?; blit_typed(registers.modify_strings(dst)?,v,filter_val); },
        InterpNatural::Bytes => { let v = registers.read_bytes(src)?; blit_typed(registers.modify_bytes(dst)?,v,filter_val); },
    }
    Ok(())
}

fn blit_number(context: &mut InterpContext, dst: &Register, src: &Register, filter: Option<&Register>, offset: usize, stride: usize) -> Result<(),String> {
    let registers = context.registers();
    let filter = filter.map(|r| registers.read_indexes(r)).transpose()?;
    let srcv = registers.read_indexes(src)?;
    let dstv = registers.modify_indexes(dst)?;
    let src = srcv.borrow();
    let mut dst = dstv.borrow_mut();
    if let Some(filter) = filter {
        let src_len = src.len();
        for (i,filter_pos) in filter.borrow().iter().enumerate() {
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
