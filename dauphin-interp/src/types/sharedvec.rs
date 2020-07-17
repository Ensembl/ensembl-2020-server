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
use crate::runtime::{ InterpContext, InterpValue, InterpValueIndexes };
use crate::types::{ VectorSource, VectorRegisters };

#[derive(Debug)]
pub struct SharedVec {
    vr: VectorRegisters,
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
