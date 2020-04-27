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

use crate::model::VectorRegisters;
use crate::interp::InterpContext;
use super::super::common::blit::{ blit_number, blit, assign_reg, blit_numbers };
use super::vectorsource::VectorSource;

pub struct VectorCopy<'a,'b,'c,'d> {
    vs_left: Box<dyn VectorSource + 'c>,
    vs_right: Box<dyn VectorSource + 'd>,
    a_left: &'a VectorRegisters,
    a_right: &'a VectorRegisters,
    filter: &'b [usize],
    lengths: Vec<(usize,usize)>
}

impl<'a,'b,'c,'d> VectorCopy<'a,'b,'c,'d> {
    pub fn new<T,U>(context: &mut InterpContext, vs_left: T, a_left: &'a VectorRegisters, vs_right: U, a_right: &'a VectorRegisters, filter: &'b [usize])
            -> Result<VectorCopy<'a,'b,'c,'d>,String>
            where T: VectorSource + 'c, U: VectorSource + 'd {
        let mut lengths = vec![];
        for level in 0..a_left.depth() {
            /* how long are the lower registers? */
            let left_lower_len = vs_left.len(context,a_left.lower_pos(level))?;
            let right_lower_len = vs_right.len(context,a_right.lower_pos(level))?;
            lengths.push((left_lower_len,right_lower_len));
        }
        Ok(VectorCopy {
            vs_left: Box::new(vs_left), vs_right: Box::new(vs_right), a_left, a_right, lengths, filter
        })
    }

    fn copy_deep(&self, context: &mut InterpContext) -> Result<(),String> {
        let copies = self.filter.len();
        let mut offsets = vec![];
        let depth = self.a_left.depth();
        /* intermediate levels */
        for level in 0..(depth-1) {
            let (start,stride) = &self.lengths[level];
            assign_reg(context,self.vs_left.as_ref(),self.a_left.offset_pos(level)?,self.vs_right.as_ref(),self.a_right.offset_pos(level)?, |mut left,right| {
                for i in 0..copies {
                    left = blit_number(left,right,None,start+i*stride)?;
                }
                Ok(left)
            })?;
            assign_reg(context,self.vs_left.as_ref(),self.a_left.length_pos(level)?,self.vs_right.as_ref(),self.a_right.length_pos(level)?, |mut left,right| {
                for _ in 0..copies {
                    left = blit_number(left,right,None,0)?;
                }
                Ok(left)
            })?;
        }
        /* bottom-level */
        assign_reg(context,self.vs_left.as_ref(),self.a_left.data_pos(),self.vs_right.as_ref(),self.a_right.data_pos(), |mut left,right| {
            for _ in 0..copies {
                left = blit(left,right,None)?;
            }
            Ok(left)
        })?;
        for i in 0..copies {
            offsets.push(self.lengths[depth-1].0+i*self.lengths[depth-1].1);
        }
        /* top level */
        assign_reg(context,self.vs_left.as_ref(),self.a_left.offset_pos(depth-1)?,self.vs_right.as_ref(),self.a_right.offset_pos(depth-1)?, |left,right| {
            blit_numbers(left,&right,Some(self.filter),&offsets)
        })?;
        assign_reg(context,self.vs_left.as_ref(),self.a_left.length_pos(depth-1)?,self.vs_right.as_ref(),self.a_right.length_pos(depth-1)?, |left,right| {
            blit_number(left,&right,Some(self.filter),0)
        })?;
        Ok(())
    }

    fn copy_shallow(&self, context: &mut InterpContext) -> Result<(),String> {
        assign_reg(context,self.vs_left.as_ref(),self.a_left.data_pos(),self.vs_right.as_ref(),self.a_right.data_pos(), |mut left,right| {
            for _ in 0..self.filter.len() {
                left = blit(left,right,Some(self.filter))?;
            }
            Ok(left)
        })?;
        Ok(())
    }

    pub fn copy(&self, context: &mut InterpContext) -> Result<(),String> {
        if self.a_left.depth() > 0 {
            self.copy_deep(context)
        } else {
            self.copy_shallow(context)
        }
    }
}
