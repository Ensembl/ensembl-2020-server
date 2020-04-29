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
use crate::model::VectorRegisters;
use crate::interp::{ InterpValue, InterpContext };
use crate::commands::common::polymorphic::arbitrate_type;
use super::sharedvec::SharedVec;
use super::vectorsource::VectorSource;

fn assign_reg<T,U>(context: &mut InterpContext, vs_left: &dyn VectorSource, left_idx: usize, right: &U, mut cb: T)
                -> Result<(),String>
                where T: FnMut(InterpValue,&U) -> Result<InterpValue,String> {
    let left = vs_left.get_exclusive(context,left_idx)?;
    vs_left.set(context,left_idx,cb(left,&right)?);
    Ok(())
}

fn blit_number(dst: InterpValue, src: &[usize], filter: Option<&[usize]>, offset: usize) -> Result<InterpValue,String> {
    let mut dstv = dst.to_indexes()?;
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

fn blit_numbers(dst: InterpValue, src: &[usize], filter: Option<&[usize]>, offsets: &[usize]) -> Result<InterpValue,String> {
    let mut dstv = dst.to_indexes()?;
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
    if let Some(natural) = arbitrate_type(&dst,src,filter_val.is_some()) {
        Ok(polymorphic!(dst,[src],natural,(|d,s| {
            blit_typed(d,s,filter_val)
        })))
    } else {
        Ok(dst)
    }
}

pub struct VectorCopy<'a,'b,'c,'d> {
    right: &'d SharedVec,
    vs_left: Box<dyn VectorSource + 'c>,
    a_left: &'a VectorRegisters,
    filter: &'b [usize],
    lengths: Vec<(usize,usize)>
}

impl<'a,'b,'c,'d> VectorCopy<'a,'b,'c,'d> {
    pub fn new<T>(context: &mut InterpContext, vs_left: T, a_left: &'a VectorRegisters, right: &'d SharedVec, filter: &'b [usize])
            -> Result<VectorCopy<'a,'b,'c,'d>,String>
            where T: VectorSource + 'c {
        let mut lengths = vec![];
        for level in 0..a_left.depth() {
            /* how long are the lower registers? */
            let left_lower_len = vs_left.len(context,a_left.lower_pos(level))?;
            let right_lower_len = if level > 0 {
                right.get_offset(level-1)?.len()
            } else {
                right.get_data().len()
            };
            lengths.push((left_lower_len,right_lower_len));
        }
        Ok(VectorCopy {
            vs_left: Box::new(vs_left), a_left, lengths, filter, right
        })
    }

    fn copy_deep(&self, context: &mut InterpContext) -> Result<(),String> {
        let copies = self.filter.len();
        let mut offsets = vec![];
        let depth = self.a_left.depth();
        /* intermediate levels */
        for level in 0..(depth-1) {
            let (start,stride) = &self.lengths[level];
            assign_reg(context,self.vs_left.as_ref(),self.a_left.offset_pos(level)?,self.right.get_offset(level)?, |mut left, right| {
                for i in 0..copies {
                    left = blit_number(left,right,None,start+i*stride)?;
                }
                Ok(left)
            })?;
            assign_reg(context,self.vs_left.as_ref(),self.a_left.length_pos(level)?,self.right.get_length(level)?, |mut left, right| {
                for _ in 0..copies {
                    left = blit_number(left,right,None,0)?;
                }
                Ok(left)
            })?;
        }
        /* bottom-level */
        self.copy_shallow(context,None,copies)?;
        for i in 0..copies {
            offsets.push(self.lengths[depth-1].0+i*self.lengths[depth-1].1);
        }
        /* top level */
        assign_reg(context,self.vs_left.as_ref(),self.a_left.offset_pos(depth-1)?,self.right.get_offset(depth-1)?, |left,right| {
            blit_numbers(left,&right,Some(self.filter),&offsets)
        })?;
        assign_reg(context,self.vs_left.as_ref(),self.a_left.length_pos(depth-1)?,self.right.get_length(depth-1)?, |left,right| {
            blit_number(left,&right,Some(self.filter),0)
        })?;
        Ok(())
    }

    fn copy_shallow(&self, context: &mut InterpContext, filter: Option<&[usize]>, len: usize) -> Result<(),String> {
        assign_reg(context,self.vs_left.as_ref(),self.a_left.data_pos(),self.right.get_data(), |mut left, right| {
            for _ in 0..len {
                left = blit(left,right,filter)?;
            }
            Ok(left)
        })?;
        Ok(())
    }

    pub fn copy(&self, context: &mut InterpContext) -> Result<(),String> {
        if self.a_left.depth() > 0 {
            self.copy_deep(context)
        } else {
            self.copy_shallow(context,Some(self.filter),self.filter.len())
        }
    }
}
