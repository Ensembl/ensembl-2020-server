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

use std::fmt;
use serde_cbor::Value as CborValue;
use crate::model::cbor_int;

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct VectorRegisters {
    start: usize,
    depth: usize
}

impl VectorRegisters {
    pub fn new(depth: usize) -> VectorRegisters {
        VectorRegisters {
            depth,
            start: 0
        }
    }

    pub fn deserialize(cbor: &CborValue) -> Result<VectorRegisters,String> {
        Ok(VectorRegisters::new(cbor_int(&cbor,None)? as usize))
    }

    pub fn serialize(&self) -> Result<CborValue,String> {
        Ok(CborValue::Integer(self.depth as i128))
    }

    pub(super) fn add_start(&mut self, start: usize) {
        self.start += start;
    }

    pub fn depth(&self) -> usize { self.depth }
    pub fn data_pos(&self) -> usize { self.start }

    pub fn lower_pos(&self, level: usize) -> usize {
        if level > 0 { self.offset_pos(level-1).unwrap() } else { self.data_pos() }
    }

    pub fn offset_pos(&self, level: usize) -> Result<usize,String> {
        if self.depth > level {
            Ok(self.start+level*2+1)
        } else {
            Err(format!("bad level {}. depth is {}",level,self.depth))
        }
    }

    pub fn length_pos(&self, level: usize) -> Result<usize,String> {
        if self.depth > level {
            Ok(self.start+level*2+2)
        } else {
            Err(format!("bad level {}. depth is {}",level,self.depth))
        }
    }

    pub fn register_count(&self) -> usize { self.depth*2+1 }
}

impl fmt::Display for VectorRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(write!(f,"<{}>",self.depth)?)
    }
}
