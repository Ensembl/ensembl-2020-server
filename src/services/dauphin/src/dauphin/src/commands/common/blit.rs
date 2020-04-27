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
use crate::interp::{ InterpValue, InterpNatural, InterpContext };
use super::vectorsource::VectorSource;

pub(super) fn blit_number(dst: InterpValue, src: &Rc<InterpValue>, filter: Option<&[usize]>, offset: usize) -> Result<InterpValue,String> {
    let srcv = src.to_rc_indexes()?.0;
    let mut dstv = dst.to_indexes()?;
    let src = &srcv;
    if let Some(filter) = filter {
        let src_len = src.len();
        for (i,filter_pos) in filter.iter().enumerate() {
            dstv[*filter_pos] = src[i%src_len] + offset;
        }
    } else {
        let mut new_values = src.iter().map(|x| *x+offset).collect();
        dstv.append(&mut new_values);
    }
    Ok(InterpValue::Indexes(dstv))
}

pub(super) fn blit_numbers(dst: InterpValue, src: &Rc<InterpValue>, filter: Option<&[usize]>, offsets: &[usize]) -> Result<InterpValue,String> {
    let srcv = src.to_rc_indexes()?.0;
    let mut dstv = dst.to_indexes()?;
    let src = &srcv;
    if offsets.len() > 0 {
        let off_len = offsets.len();
        if let Some(filter) = filter {
            let src_len = src.len();
            for (i,filter_pos) in filter.iter().enumerate() {
                dstv[*filter_pos] = src[i%src_len] + offsets[i%off_len];
            }
        } else {
            for (i,val) in src.iter().enumerate() {
                dstv.push(val+offsets[i%off_len]);
            }
        }
    }
    Ok(InterpValue::Indexes(dstv))
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
#[macro_use]
macro_rules! run_typed2 {
    ($dst:expr,$src:expr,$natural:expr,$func:tt) => {
        match $natural {
            $crate::interp::InterpNatural::Empty => { $dst },
            $crate::interp::InterpNatural::Numbers => { let s = $src.to_rc_numbers()?.0; let mut d = $dst.to_numbers()?; $func(&mut d,&s); $crate::interp::InterpValue::Numbers(d) },
            $crate::interp::InterpNatural::Indexes => { let s = $src.to_rc_indexes()?.0; let mut d = $dst.to_indexes()?; $func(&mut d,&s); $crate::interp::InterpValue::Indexes(d) },
            $crate::interp::InterpNatural::Boolean => { let s = $src.to_rc_boolean()?.0; let mut d = $dst.to_boolean()?; $func(&mut d,&s); $crate::interp::InterpValue::Boolean(d) },
            $crate::interp::InterpNatural::Strings => { let s = $src.to_rc_strings()?.0; let mut d = $dst.to_strings()?; $func(&mut d,&s); $crate::interp::InterpValue::Strings(d) },
            $crate::interp::InterpNatural::Bytes => { let s = $src.to_rc_bytes()?.0; let mut d = $dst.to_bytes()?; $func(&mut d,&s); $crate::interp::InterpValue::Bytes(d) },
        }
    };
}

#[macro_use]
macro_rules! run_typed2a {
    ($dst:expr,$src:expr,$natural:expr,$func:tt) => {
        match $natural {
            $crate::interp::InterpNatural::Empty => { None },
            $crate::interp::InterpNatural::Numbers => { let s = $src.to_rc_numbers()?.0; let d = $dst.to_rc_numbers()?.0; Some($func(&d,&s)) },
            $crate::interp::InterpNatural::Indexes => { let s = $src.to_rc_indexes()?.0; let d = $dst.to_rc_indexes()?.0; Some($func(&d,&s)) },
            $crate::interp::InterpNatural::Boolean => { let s = $src.to_rc_boolean()?.0; let d = $dst.to_rc_boolean()?.0; Some($func(&d,&s)) },
            $crate::interp::InterpNatural::Strings => { let s = $src.to_rc_strings()?.0; let d = $dst.to_rc_strings()?.0; Some($func(&d,&s)) },
            $crate::interp::InterpNatural::Bytes => { let s = $src.to_rc_bytes()?.0; let d = $dst.to_rc_bytes()?.0; Some($func(&d,&s)) },
        }
    };
}

fn blit_typed<T>(dst: &mut Vec<T>, src: &Vec<T>, filter: Option<&[usize]>) where T: Clone {
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

pub(super) fn blit(dst: InterpValue, src: &Rc<InterpValue>, filter_val: Option<&[usize]>) -> Result<InterpValue,String> {
    if let Some(natural) = coerce_to(&dst,src,filter_val.is_some()) {
        Ok(run_typed2!(dst,src,natural,(|d,s| {
            blit_typed(d,s,filter_val)
        })))
    } else {
        Ok(dst)
    }
}

pub(super) fn assign_reg<T>(context: &mut InterpContext, vs_left: &dyn VectorSource, left_idx: usize, vs_right: &dyn VectorSource, right_idx: usize, mut cb: T)
                -> Result<(),String>
                where T: FnMut(InterpValue,&Rc<InterpValue>) -> Result<InterpValue,String> {
    let right = vs_right.get_shared(context,right_idx)?;
    let left = vs_left.get_exclusive(context,left_idx)?;
    vs_left.set(context,left_idx,cb(left,&right)?);
    Ok(())
}
