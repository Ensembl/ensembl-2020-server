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
use crate::interp::{ InterpValue, InterpContext };
use crate::commands::common::polymorphic::arbitrate_type;
use super::sharedvec::SharedVec;
use super::writevec::WriteVec;
use super::vectorsource::RegisterVectorSource;
use crate::model::VectorRegisters;

pub fn vector_update_lengths(dst: &mut Vec<usize>, src: &[usize], filter: &[usize]) {
    let mut src_it = src.iter().cycle();
    for filter_pos in filter.iter() {
        dst[*filter_pos] = *src_it.next().unwrap();
    }
}

pub fn vector_update_offsets(dst: &mut Vec<usize>, src: &[usize], filter: &[usize], offsets: (usize,usize)) {
    let mut src_it = src.iter().cycle();
    let mut offset = offsets.0;
    for filter_pos in filter.iter() {
        dst[*filter_pos] = *src_it.next().unwrap() + offset;
        offset += offsets.1;
    }
}

pub fn vector_append<F,T>(dst: &mut Vec<T>, src: &[T], mut cb: F) where F: FnMut(&T) -> T {
    let mut new_values = src.iter().map(|x| cb(x)).collect();
    dst.append(&mut new_values);
}

pub fn vector_append_lengths(dst: &mut Vec<usize>, src: &[usize]) {
    dst.append(&mut src.to_vec());
}

pub fn vector_append_offsets(dst: &mut Vec<usize>, src: &[usize], delta: usize) {
    let mut src = src.to_vec();
    for v in &mut src {
        *v += delta;
    }
    dst.append(&mut src);
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

pub fn vector_push<'e>(left: &mut WriteVec<'e>, right: &SharedVec, copies: usize) -> Result<(usize,usize),String> {
    let depth = left.depth();
    /* data for top-level */
    let start = if depth > 1 { left.get_offset(depth-2)?.len() } else { left.get_data().len() };
    let stride = if depth > 1 { right.get_offset(depth-2)?.len() } else { right.get_data().len() };
    /* intermediate levels */
    for level in (0..(depth-1)).rev() {
        let start = if level > 0 { left.get_offset(level-1)?.len() } else { left.get_data().len() };
        let stride = if level > 0 { right.get_offset(level-1)?.len() } else { right.get_data().len() };
        for i in 0..copies {
            vector_append_offsets(left.get_offset_mut(level)?,right.get_offset(level)?,start+i*stride);
            vector_append_lengths(left.get_length_mut(level)?,right.get_length(level)?);
        }
    }
    /* bottom-level */
    let mut leftdata = left.take_data()?;
    let rightdata = right.get_data();
    leftdata = append_data(leftdata,&rightdata,copies)?.0;
    left.replace_data(leftdata)?;
    Ok((start,stride))
}

pub fn vector_register_copy<'e>(context: &mut InterpContext, rvs: &RegisterVectorSource<'e>, dst: &VectorRegisters, src: &VectorRegisters) -> Result<(),String> {
    for level in 0..dst.depth() {
        rvs.copy(context,dst.offset_pos(level)?,src.offset_pos(level)?)?;
        rvs.copy(context,dst.length_pos(level)?,src.length_pos(level)?)?;
    }
    rvs.copy(context,dst.data_pos(),src.data_pos())?;
    Ok(())
}