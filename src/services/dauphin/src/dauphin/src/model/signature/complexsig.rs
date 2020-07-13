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
use super::super::definitionstore::DefStore;
use super::super::structenum::{ EnumDef, StructDef };
use super::complexpath::ComplexPath;
use super::vectorsig::VectorRegisters;
use crate::model::{ cbor_array, cbor_int };
use crate::typeinf::{ BaseType, ContainerType, MemberType, MemberMode };
use serde_cbor::Value as CborValue;
use super::fulltype::FullType;

pub struct ComplexRegisters(FullType);

impl ComplexRegisters {
    fn new_empty(mode: MemberMode) -> ComplexRegisters {
        ComplexRegisters(FullType::new_empty(mode))
    }

    fn new(defstore: &DefStore, mode: MemberMode, type_: &MemberType) -> Result<FullType,String> {
        let mut out = ComplexRegisters::new_empty(mode);
        out.vec_from_type(defstore,type_,&ComplexPath::new_empty(),&ContainerType::new_empty())?;
        Ok(out.0)
    }

    fn vec_from_type(&mut self, defstore: &DefStore, type_: &MemberType, path: &ComplexPath, container: &ContainerType) -> Result<(),String> {
        let path = path.add_levels(type_.get_container().depth());
        let container = container.merge(&type_.get_container());
        match type_.get_base() {
            BaseType::StructType(name) => {
                let struct_ = defstore.get_struct_id(&name)?;
                self.from_struct(defstore,struct_,&path,&container)
            },
            BaseType::EnumType(name) => {
                let enum_ = defstore.get_enum_id(&name)?;
                self.from_enum(defstore,enum_,&path,&container)
            },
            base => {
                self.0.add(path.clone(),VectorRegisters::new(container.depth(),base));
                Ok(())
            }
        }
    }

    fn from_struct(&mut self, defstore: &DefStore, se: &StructDef, cpath: &ComplexPath, container: &ContainerType) -> Result<(),String> {
        for name in se.get_names() {
            let new_cpath = cpath.add(se.identifier(),name);
            let type_ = se.get_member_type(name).unwrap();
            self.vec_from_type(defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }

    fn from_enum(&mut self, defstore: &DefStore, se: &EnumDef, cpath: &ComplexPath, container: &ContainerType) -> Result<(),String> {
        self.0.add(cpath.clone(),VectorRegisters::new(container.depth(),BaseType::NumberType));
        for name in se.get_names() {
            let new_cpath = cpath.add(se.identifier(),name);
            let type_ = se.get_branch_type(name).unwrap();
            self.vec_from_type(defstore,&type_,&new_cpath,container)?;
        }
        Ok(())
    }
}

pub fn make_full_type(defstore: &DefStore, mode: MemberMode, type_: &MemberType) -> Result<FullType,String> {
    ComplexRegisters::new(defstore,mode,type_)
}