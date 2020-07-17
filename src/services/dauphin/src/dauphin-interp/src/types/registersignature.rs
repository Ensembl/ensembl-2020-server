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

use std::iter::Iterator;
use std::ops::Index;
use std::slice::SliceIndex;
use crate::util::cbor::cbor_array;
use crate::types::FullType;
use serde_cbor::Value as CborValue;

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct RegisterSignature {
    index: usize,
    args: Vec<FullType>
}

impl RegisterSignature {
    pub fn new() -> RegisterSignature {
        RegisterSignature {
            index: 0,
            args: Vec::new()
        }
    }

    pub fn add(&mut self, mut cr: FullType) {
        cr.add_start(self.index);
        self.index += cr.register_count();
        self.args.push(cr);
    }

    pub fn iter<'a>(&'a self) -> RegisterSignatureIterator<'a> {
        RegisterSignatureIterator {
            rs: self,
            index: 0
        }
    }

    pub fn serialize(&self, named: bool) -> Result<CborValue,String> {
        Ok(CborValue::Array(self.args.iter().map(|x| x.serialize(named)).collect::<Result<Vec<_>,_>>()?))
    }

    pub fn deserialize(cbor: &CborValue, named: bool) -> Result<RegisterSignature,String> {
        let mut out = RegisterSignature::new();
        for cr in cbor_array(cbor,0,true)?.iter().map(|x| FullType::deserialize(x,named)).collect::<Result<Vec<_>,_>>()? {
            out.add(cr);
        }
        Ok(out)
    }
}

pub struct RegisterSignatureIterator<'a> {
    rs: &'a RegisterSignature,
    index: usize
}

impl<'a> Iterator for RegisterSignatureIterator<'a> {
    type Item = &'a FullType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.rs.args.len() {
            let out = Some(&self.rs.args[self.index]);
            self.index += 1;
            out
        } else {
            None
        }
    }
}

impl<I> Index<I> for RegisterSignature where I: SliceIndex<[FullType]> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.args.index(index)
    }
}
