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

use std::collections::{ HashMap, HashSet };
use std::hash::{ Hash, Hasher };
use std::fmt;
use super::complexpath::ComplexPath;
use crate::types::{ MemberMode, VectorRegisters };
use crate::util::cbor::{ cbor_array };
use serde_cbor::Value as CborValue;

#[derive(Clone,Debug,Eq)]
pub struct FullType {
    start: usize,
    mode: MemberMode,
    order: Vec<ComplexPath>,
    vectors: HashMap<ComplexPath,VectorRegisters>
}

impl PartialEq for FullType {
    fn eq(&self, other: &Self) -> bool {
        if self.start != other.start || self.mode != other.mode || self.order != other.order {
            return false;
        }
        for path in self.order.iter() {
            if self.vectors.get(path) != other.vectors.get(path) {
                return false;
            }
        }
        true
    }
}

impl Hash for FullType {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.start.hash(hasher);
        self.mode.hash(hasher);
        self.order.hash(hasher);
        for path in self.order.iter() {
            self.vectors.get(path).hash(hasher);
        }
    }
}

impl fmt::Display for FullType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.iter().map(|x| {
            let parts = x.0.to_string();
            format!("{}{}",parts,x.1.to_string())
        }).collect::<Vec<_>>().join(",");
        write!(f,"{}/{}",s,self.mode)
    }
}

impl FullType {
    pub fn new_empty(mode: MemberMode) -> FullType {
        FullType {
            mode,
            start: 0,
            order: Vec::new(),
            vectors: HashMap::new()
        }
    }

    pub fn all_registers(&self) -> HashSet<usize> {
        let mut out = HashSet::new();
        for vr in self.vectors.values() {
            out.extend(vr.all_registers());
        }
        out
    }

    pub fn get_root_path(&self) -> Result<&ComplexPath,String> {
        self.order.get(0).ok_or_else(|| format!("no root present"))
    }

    pub fn get_vec_depth(&self, path: &ComplexPath) -> Result<usize,String> {
        Ok(path.get_breaks().iter().sum())
    }

    pub fn deserialize(cbor: &CborValue, named: bool) -> Result<FullType,String> {
        let data = cbor_array(cbor,1,true)?;
        let mut out = FullType::new_empty(MemberMode::deserialize(&data[0])?);
        let mut mult = 1;
        if named { mult +=1; }
        let len = (data.len()-1)/mult;
        if len*mult+1 != data.len() {
            return Err(format!("malformed complexregisters cbor"));
        }
        for i in 0..len {
            let vs = VectorRegisters::deserialize(&data[i*mult+1])?;
            let path = if named {
                ComplexPath::deserialize(&data[i*mult+2])?
            } else {
                ComplexPath::new_anon()
            };
            out.add(path,vs);
        }
        Ok(out)
    }

    pub fn serialize(&self, named: bool) -> Result<CborValue,String> {
        let mut regs = vec![self.mode.serialize()];
        for complex in &self.order {
            regs.push(self.vectors.get(complex).as_ref().unwrap().serialize(false)?);
            if named {
                regs.push(complex.serialize()?);
            }
        }
        Ok(CborValue::Array(regs))
    }

    pub fn add_start(&mut self, start: usize) {
        for (_,vr) in self.vectors.iter_mut() {
            vr.add_start(start);
        }
        self.start += start;
    }

    pub fn get_mode(&self) -> MemberMode { self.mode }

    pub fn add(&mut self, complex: ComplexPath, mut vr: VectorRegisters) {
        vr.add_start(self.start);
        self.start += vr.register_count();
        self.order.push(complex.clone());
        self.vectors.insert(complex.clone(),vr);
    }

    pub fn iter<'a>(&'a self) -> FullTypeIterator<'a> {
        FullTypeIterator {
            cr: self,
            index: 0
        }
    }

    pub fn register_count(&self) -> usize {
        self.iter().map(|x| x.1.register_count()).sum()
    }

}

pub struct FullTypeIterator<'a> {
    cr: &'a FullType,
    index: usize
}

impl<'a> Iterator for FullTypeIterator<'a> {
    type Item = (&'a ComplexPath,&'a VectorRegisters);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.cr.order.len() {
            let name = &self.cr.order[self.index];
            if let Some(out) = self.cr.vectors.get(name) {
                self.index += 1;
                return Some((name,out));
            }
        }
        None
    }
}
