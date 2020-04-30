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

pub fn vector_update<F,T>(dst: &mut Vec<T>, src: &[T], filter: &[usize], mut cb: F) where F: FnMut(&T) -> T {
    let src_len = src.len();
    for (i,filter_pos) in filter.iter().enumerate() {
        dst[*filter_pos] = cb(&src[i%src_len]);
    }
}

pub fn vector_append<F,T>(dst: &mut Vec<T>, src: &[T], mut cb: F) where F: FnMut(&T) -> T {
    let mut new_values = src.iter().map(|x| cb(x)).collect();
    dst.append(&mut new_values);
}

fn update_poly<T>(dst: &mut Vec<T>, src: &Vec<T>, filter: &[usize]) where T: Clone {
    vector_update(dst,src,filter,|v| v.clone())
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

pub fn append_data(dst: InterpValue, src: &Rc<InterpValue>) -> Result<(InterpValue,usize),String> {
    let offset = src.len();
    if let Some(natural) = arbitrate_type(&dst,src,false) {
        Ok((polymorphic!(dst,[src],natural,(|d: &mut Vec<_>, s: &[_]| {
            d.append(&mut s.to_vec());
        })),offset))
    } else {
        Ok((dst,offset))
    }
}

pub fn vector_push<'e>(left: &mut WriteVec<'e>, right: &SharedVec, copies: usize) -> Result<Vec<usize>,String> {
    let depth = left.depth();
    /* data for top-level */
    let mut offsets = vec![];
    let start = if depth > 1 { left.get_offset(depth-2)?.len() } else { left.get_data().len() };
    let stride = if depth > 1 { right.get_offset(depth-2)?.len() } else { right.get_data().len() };
    for i in 0..copies {
        offsets.push(start+i*stride);
    }
    /* intermediate levels */
    for level in (0..(depth-1)).rev() {
        let start = if level > 0 { left.get_offset(level-1)?.len() } else { left.get_data().len() };
        let stride = if level > 0 { right.get_offset(level-1)?.len() } else { right.get_data().len() };
        for i in 0..copies {
            vector_append(left.get_offset_mut(level)?,right.get_offset(level)?,|v| *v+start+i*stride);
            vector_append(left.get_length_mut(level)?,right.get_length(level)?,|v| *v);
        }
    }
    /* bottom-level */
    for _ in 0..copies {
        let data = append_data(left.take_data()?,right.get_data())?.0;
        left.replace_data(data)?;
    }
    Ok(offsets)
}

pub fn vector_register_copy<'e>(context: &mut InterpContext, rvs: &RegisterVectorSource<'e>, dst: &VectorRegisters, src: &VectorRegisters) -> Result<(),String> {
    for level in 0..dst.depth() {
        rvs.copy(context,dst.offset_pos(level)?,src.offset_pos(level)?)?;
        rvs.copy(context,dst.length_pos(level)?,src.length_pos(level)?)?;
    }
    rvs.copy(context,dst.data_pos(),src.data_pos())?;
    Ok(())
}