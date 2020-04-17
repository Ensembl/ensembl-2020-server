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

use std::collections::HashMap;  // XXX to hashbrown
use std::fmt;
use super::super::definitionstore::DefStore;
use super::super::structenum::{ EnumDef, StructDef };
use super::vectorsig::VectorRegisters;
use crate::model::{ cbor_array, cbor_string };
use crate::typeinf::{ BaseType, ContainerType, MemberType, MemberMode, MemberDataFlow };
use serde_cbor::Value as CborValue;

#[derive(Clone,Debug,PartialEq)]
pub struct ComplexRegisters {
    start: usize,
    mode: MemberMode,
    flow: MemberDataFlow,
    order: Vec<Vec<String>>,
    vectors: HashMap<Vec<String>,VectorRegisters>
}

impl fmt::Display for ComplexRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.iter().map(|x| {
            let parts = x.0.iter().map(|x| format!("{}",x)).collect::<Vec<_>>().join(".");
            format!("{}{}",parts,x.1.to_string())
        }).collect::<Vec<_>>().join(",");
        write!(f,"{}/{}",s,self.mode)
    }
}

impl ComplexRegisters {
    fn new_empty(mode: MemberMode, flow: MemberDataFlow) -> ComplexRegisters {
        ComplexRegisters {
            mode, flow,
            start: 0,
            order: Vec::new(),
            vectors: HashMap::new()
        }
    }

    pub fn new(defstore: &DefStore, mode: MemberMode, type_: &MemberType, flow: MemberDataFlow) -> Result<ComplexRegisters,String> {
        let mut out = ComplexRegisters::new_empty(mode,flow);
        out.vec_from_type(defstore,type_,&vec![],&ContainerType::new_empty())?;
        Ok(out)
    }

    pub fn deserialize(cbor: &CborValue, named: bool) -> Result<ComplexRegisters,String> {
        let data = cbor_array(cbor,2,true)?;
        let mut out = ComplexRegisters::new_empty(MemberMode::deserialize(&data[0])?,MemberDataFlow::deserialize(&data[1])?);
        for (i,member) in cbor_array(&data[2],0,true)?.iter().enumerate() {
            if named {
                let entry = cbor_array(member,2,false)?;
                let name = cbor_array(&entry[0],0,true)?.iter().map(|x| cbor_string(x)).collect::<Result<Vec<_>,_>>()?;
                let vs = VectorRegisters::deserialize(&entry[1])?;
                out.add(name,vs);
            } else {
                let vs = VectorRegisters::deserialize(&member)?;
                out.add(vec![format!("anon-{}",i)],vs);
            }
        }
        Ok(out)
    }

    pub fn serialize(&self, named: bool) -> Result<CborValue,String> {
        let mut regs = vec![];
        if named {
            for complex in &self.order {
                regs.push(CborValue::Array(vec![
                    CborValue::Array(complex.iter().map(|x| CborValue::Text(x.to_string())).collect()),
                    self.vectors.get(complex).as_ref().unwrap().serialize()?
                ]));
            }
        } else {
            for complex in &self.order {
                regs.push(self.vectors.get(complex).as_ref().unwrap().serialize()?);
            }
        }
        Ok(CborValue::Array(vec![self.mode.serialize(),self.flow.serialize(),CborValue::Array(regs)]))
    }

    pub fn add_start(&mut self, start: usize) {
        for (_,vr) in self.vectors.iter_mut() {
            vr.add_start(start);
        }
        self.start += start;
    }

    pub fn get_mode(&self) -> MemberMode { self.mode }

    pub fn justifies_call(&self) -> bool {
        if let MemberDataFlow::JustifiesCall = self.flow { true } else { false }
    }

    fn add(&mut self, complex: Vec<String>, mut vr: VectorRegisters) {
        vr.add_start(self.start);
        self.start += vr.register_count();
        self.order.push(complex.to_vec());
        self.vectors.insert(complex,vr);
    }

    pub fn iter<'a>(&'a self) -> ComplexRegistersIterator<'a> {
        ComplexRegistersIterator {
            cr: self,
            index: 0
        }
    }

    pub fn register_count(&self) -> usize {
        self.iter().map(|x| x.1.register_count()).sum()
    }

    pub(super) fn vec_from_type(&mut self, defstore: &DefStore, type_: &MemberType, prefix: &[String], container: &ContainerType) -> Result<(),String> {
        let container = container.merge(&type_.get_container());
        match type_.get_base() {
            BaseType::StructType(name) => {
                let struct_ = defstore.get_struct(&name).unwrap();
                self.from_struct(defstore,struct_,prefix,&container)
            },
            BaseType::EnumType(name) => {
                let enum_ = defstore.get_enum(&name).unwrap();
                self.from_enum(defstore,enum_,prefix,&container)
            },
            base => {
                self.add(prefix.to_vec(),VectorRegisters::new(container.depth(),base));
                Ok(())
            }
        }
    }

    fn from_struct(&mut self, defstore: &DefStore, se: &StructDef, cpath: &[String], container: &ContainerType) -> Result<(),String> {
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_member_type(name).unwrap();
            self.vec_from_type(defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }

    fn from_enum(&mut self, defstore: &DefStore, se: &EnumDef, cpath: &[String], container: &ContainerType) -> Result<(),String> {
        self.add(cpath.to_vec(),VectorRegisters::new(container.depth(),BaseType::NumberType));
        for name in se.get_names() {
            let mut new_cpath = cpath.to_vec();
            new_cpath.push(name.to_string());
            let type_ = se.get_branch_type(name).unwrap();
            self.vec_from_type(defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }

}

pub struct ComplexRegistersIterator<'a> {
    cr: &'a ComplexRegisters,
    index: usize
}

impl<'a> Iterator for ComplexRegistersIterator<'a> {
    type Item = (&'a Vec<String>,&'a VectorRegisters);

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
