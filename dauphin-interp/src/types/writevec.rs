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

use std::mem::replace;
use crate::runtime::{ InterpContext, InterpValue };
use crate::types::{ VectorRegisters, VectorSource };

pub struct WriteVec<'a> {
    vs: &'a dyn VectorSource,
    vr: VectorRegisters,
    structure: Vec<(Vec<usize>,Vec<usize>)>,
    data: InterpValue
}

impl<'a> WriteVec<'a> {
    pub fn new(context: &mut InterpContext, vs: &'a dyn VectorSource, vr: &VectorRegisters) -> Result<WriteVec<'a>,String> {
        let mut structs = vec![];
        for level in 0..vr.depth() {
            structs.push((
                vs.get_exclusive(context,vr.offset_pos(level)?)?.to_indexes()?,
                vs.get_exclusive(context,vr.length_pos(level)?)?.to_indexes()?
            ));
        }
        let data = vs.get_exclusive(context,vr.data_pos())?;
        Ok(WriteVec {
            vs,
            vr: vr.clone(),
            structure: structs,
            data
        })
    }

    pub fn depth(&self) -> usize { self.structure.len() }

    pub fn get_data(&self) -> &InterpValue { &self.data }

    fn get(&self, level: usize) -> Result<&(Vec<usize>,Vec<usize>),String> {
        self.structure.get(level).ok_or_else(|| format!("index out of range"))
    }

    pub fn get_offset(&self, level: usize) -> Result<&Vec<usize>,String> {
        self.get(level).map(|x| &x.0)
    }

    pub fn get_length(&self, level: usize) -> Result<&Vec<usize>,String> {
        self.get(level).map(|x| &x.1)
    }

    fn get_mut(&mut self, level: usize) -> Result<&mut (Vec<usize>,Vec<usize>),String> {
        self.structure.get_mut(level).ok_or_else(|| format!("index out of range"))
    }

    pub fn get_offset_mut(&mut self, level: usize) -> Result<&mut Vec<usize>,String> {
        Ok(&mut self.get_mut(level)?.0)
    }

    pub fn get_length_mut(&mut self, level: usize) -> Result<&mut Vec<usize>,String> {
        Ok(&mut self.get_mut(level)?.1)
    }

    pub fn take_data(&mut self) -> Result<InterpValue,String> {
        let out = replace(&mut self.data, InterpValue::Empty);
        Ok(out)
    }

    pub fn replace_data(&mut self, data: InterpValue) -> Result<(),String> {
        self.data = data;
        Ok(())
    }

    pub fn write(&mut self, context: &mut InterpContext) -> Result<(),String> {
        self.vs.set(context,self.vr.data_pos(),replace(&mut self.data,InterpValue::Empty));
        for (level,(offsetr,lengthr)) in self.structure.drain(..).enumerate() {
            self.vs.set(context,self.vr.offset_pos(level)?,InterpValue::Indexes(offsetr));
            self.vs.set(context,self.vr.length_pos(level)?,InterpValue::Indexes(lengthr));
        }
        Ok(())
    }
}
