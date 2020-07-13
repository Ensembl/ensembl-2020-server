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
use std::rc::Rc;
use std::cell::RefCell;
use serde_cbor::Value as CborValue;

#[derive(Clone,Copy,Hash,PartialEq,Eq,PartialOrd,Ord)]
pub struct Register(pub usize);

impl Register {
    pub fn deserialize(v: &CborValue) -> Result<Register,String> {
        if let CborValue::Integer(r) = v {
            Ok(Register(*r as usize))
        } else {
            Err("bad cbor, expected register".to_string())
        }
    }

    pub fn serialize(&self) -> CborValue {
        CborValue::Integer(self.0 as i128)
    }
}

impl fmt::Debug for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"%{}",self.0)?;
        Ok(())
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"%{}",self.0)?;
        Ok(())
    }
}
