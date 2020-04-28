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

use std::fmt::Debug;
use std::rc::Rc;
use crate::interp::InterpContext;
use crate::interp::{ InterpValue, InterpValueIndexes, InterpValueNumbers };
use crate::model::VectorRegisters;
use super::vectorsource::VectorSource;
use super::super::common::blit::coerce_to;

#[derive(Debug)]
pub struct SharedVec {
    vr:  VectorRegisters,
    structure: Vec<(InterpValueIndexes,InterpValueIndexes)>,
    data: Rc<InterpValue>
}

impl SharedVec {
    pub fn new(context: &mut InterpContext, vs: &dyn VectorSource, vr: &VectorRegisters) -> Result<SharedVec,String> {
        let mut structs = vec![];
        for level in 0..vr.depth() {
            structs.push((
                vs.get_shared(context,vr.offset_pos(level)?)?.to_rc_indexes()?.0,
                vs.get_shared(context,vr.length_pos(level)?)?.to_rc_indexes()?.0
            ));
        }
        Ok(SharedVec {
            vr: vr.clone(),
            structure: structs,
            data: vs.get_shared(context,vr.data_pos())?
        })
    }

    pub fn depth(&self) -> usize { self.structure.len() }

    pub fn get_data(&self) -> &Rc<InterpValue> { &self.data }

    fn get(&self, level: usize) -> Result<&(InterpValueIndexes,InterpValueIndexes),String> {
        self.structure.get(level).ok_or_else(|| format!("index out of range"))
    }

    pub fn get_offset(&self, level: usize) -> Result<&InterpValueIndexes,String> {
        self.get(level).map(|x| &x.0)
    }

    pub fn get_length(&self, level: usize) -> Result<&InterpValueIndexes,String> {
        self.get(level).map(|x| &x.1)
    }
}

fn compare_work<T>(a: &SharedVec, a_off: (usize,usize), a_data: &[T], b: &SharedVec, b_off: (usize,usize), b_data: &[T], level: usize) -> Result<bool,String>
        where T: PartialEq {
    if a_off.1 != b_off.1 { return Ok(false); }
    if level > 0 {
        /* index with index below */
        let lower_a_off = a.get_offset(level-1)?;
        let lower_a_len = a.get_length(level-1)?;
        let lower_b_off = b.get_offset(level-1)?;
        let lower_b_len = b.get_length(level-1)?;
        for i in 0..a_off.1 {
            if !compare_work(a,(lower_a_off[a_off.0+i],lower_a_len[a_off.0+i]),a_data,
                                b,(lower_b_off[b_off.0+i],lower_b_len[b_off.0+i]),b_data,
                                level-1)? {
                return Ok(false);
            }
        }
    } else {
        /* index with data below */
        for i in 0..a_off.1 {
            if a_data[a_off.0+i] != b_data[b_off.0+i] {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn compare_indexed<T>(a: &SharedVec, b: &SharedVec, a_data: &[T], b_data: &[T]) -> Result<Vec<bool>,String> where T: PartialEq + Debug {
    let top_a_off = a.get_offset(a.depth()-1)?;
    let top_a_len = a.get_length(a.depth()-1)?;
    let top_b_off = b.get_offset(b.depth()-1)?;
    let top_b_len = b.get_length(b.depth()-1)?;
    let b_len = top_b_off.len();
    let mut out = vec![];
    for i in 0..top_a_off.len() {
        out.push(compare_work(a,(top_a_off[i],top_a_len[i]),a_data,
                              b,(top_b_off[i%b_len],top_b_len[i%b_len]),b_data,
                              a.depth()-1)?);
    }
    Ok(out)
}

fn compare_data<T>(a: &[T], b: &[T]) -> Vec<bool> where T: PartialEq {
    let b_len = b.len();
    a.iter().enumerate().map(|(i,av)| av == &b[i%b_len]).collect()
}

pub fn compare(a: &SharedVec, b: &SharedVec) -> Result<Vec<bool>,String> {
    if a.depth() != b.depth() {
        return Err(format!("unequal types in eq"));
    }
    if a.depth() != 0 {
        /* indexed */
        let a_data = a.get_data();
        let b_data = b.get_data();
        if let Some(natural) = coerce_to(&a_data,&b_data,true) {
            Ok(run_typed2a!(&a_data,&b_data,natural,(|d,s| {
                compare_indexed(a,b,d,s)
            })).transpose()?.ok_or_else(|| format!("unexpected empty in eq"))?)
        } else {
            Ok(vec![])
        }
    } else {
        /* data */
        let a_data = a.get_data();
        let b_data = b.get_data();
        if let Some(natural) = coerce_to(&a_data,&b_data,true) {
            Ok(run_typed2a!(&a_data,&b_data,natural,(|d,s| {
                compare_data(d,s)
            })).ok_or_else(|| format!("unexpected empty in eq"))?)
        } else {
            Ok(vec![])
        }
    }
}
