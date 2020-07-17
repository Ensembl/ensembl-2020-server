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

use std::collections::HashSet;
use std::fmt;

use crate::typeinf::{ MemberType };
use dauphin_interp::command::Identifier;

pub struct StructEnumDef {
    type_: String,
    identifier: Identifier,
    names: Vec<String>,
    member_types: Vec<MemberType>
}

impl fmt::Debug for StructEnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{} {} {{ ",self.type_,self.identifier)?;
        for (i,t) in self.member_types.iter().enumerate() {
            if i > 0 { write!(f,", ")?; }
            write!(f,"{}: ",self.names[i])?;
            write!(f,"{:?}",t)?;
        }
        write!(f," }}")?;
        Ok(())
    }
}

fn no_duplicates(input: &Vec<String>) -> Result<(),String> { // TODO test
    let mut seen = HashSet::new();
    for name in input {
        if seen.contains(name) {
            return Err(format!("Duplicate name: '{:?}'",name));
        }
        seen.insert(name);
    }
    Ok(())
}

impl StructEnumDef {
    pub fn new(type_: &str, identifier: &Identifier, member_types: &Vec<MemberType>, names: &Vec<String>) -> Result<StructEnumDef,String> {
        no_duplicates(names)?;
        Ok(StructEnumDef {
            type_: type_.to_string(),
            identifier: identifier.clone(),
            names: names.clone(),
            member_types: member_types.clone()
        })
    }

    //

    pub fn identifier(&self) -> &Identifier { &self.identifier }
    pub fn get_names(&self) -> &Vec<String> { &self.names }

    pub fn type_from_name(&self, name: &String) -> Option<MemberType> {
        for (i,this_name) in self.names.iter().enumerate() {
            if this_name == name {
                return Some(self.member_types[i].clone());
            }
        }
        None
    }

    pub fn get_types(&self) -> &Vec<MemberType> {
        &self.member_types
    }
}

pub struct StructDef {
    common: StructEnumDef
}

impl StructDef {
    pub fn new(identifier: &Identifier, member_types: &Vec<MemberType>, names: &Vec<String>) -> Result<StructDef,String> {
        Ok(StructDef {
            common: StructEnumDef::new("struct",identifier,member_types,names)?
        })
    }

    pub fn identifier(&self) -> &Identifier { &self.common.identifier() }
    pub fn get_names(&self) -> &Vec<String> { &self.common.get_names() }

    pub fn get_member_type(&self, name: &String) -> Option<MemberType> {
        self.common.type_from_name(name)
    }

    pub fn get_member_types(&self) -> &Vec<MemberType> {
        self.common.get_types()
    }
}

impl fmt::Debug for StructDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.common)
    }
}

pub struct EnumDef {
    common: StructEnumDef
}

impl EnumDef {
    pub fn new(identifier: &Identifier, member_types: &Vec<MemberType>, names: &Vec<String>) -> Result<EnumDef,String> {
        Ok(EnumDef {
            common: StructEnumDef::new("enum",identifier,member_types,names)?
        })
    }

    pub fn identifier(&self) -> &Identifier { &self.common.identifier() }
    pub fn get_names(&self) -> &Vec<String> { &self.common.get_names() }

    pub fn get_branch_type(&self, name: &String) -> Option<MemberType> {
        self.common.type_from_name(name)
    }

    pub fn get_branch_types(&self) -> &Vec<MemberType> {
        self.common.get_types()
    }
}

impl fmt::Debug for EnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.common)
    }
}
