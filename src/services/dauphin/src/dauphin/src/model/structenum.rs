use std::collections::HashSet;
use std::fmt;

use crate::typeinf::{ MemberType };

pub struct StructEnumDef {
    type_: String,
    name: String,
    names: Vec<String>,
    member_types: Vec<MemberType>
}

impl fmt::Debug for StructEnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{} {} {{ ",self.type_,self.name)?;
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
        seen.insert(name.to_string());
    }
    Ok(())
}

impl StructEnumDef {
    pub fn new(type_: &str, name: &str, member_types: &Vec<MemberType>, names: &Vec<String>) -> Result<StructEnumDef,String> {
        no_duplicates(names)?;
        Ok(StructEnumDef {
            type_: type_.to_string(),
            name: name.to_string(),
            names: names.clone(),
            member_types: member_types.clone()
        })
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn get_names(&self) -> &Vec<String> { &self.names }

    pub fn type_from_name2(&self, name: &str) -> Option<MemberType> {
        for (i,this_name) in self.names.iter().enumerate() {
            if this_name == name {
                return Some(self.member_types[i].clone());
            }
        }
        None
    }

    pub fn get_types2(&self) -> &Vec<MemberType> {
        &self.member_types
    }
}

pub struct StructDef {
    common: StructEnumDef
}

impl StructDef {
    pub fn new(name: &str, member_types: &Vec<MemberType>, names: &Vec<String>) -> Result<StructDef,String> {
        Ok(StructDef {
            common: StructEnumDef::new("struct",name,member_types,names)?
        })
    }

    pub fn name(&self) -> &str { &self.common.name() }
    pub fn get_names(&self) -> &Vec<String> { &self.common.get_names() }

    pub fn get_member_type(&self, name: &str) -> Option<MemberType> {
        self.common.type_from_name2(name)
    }

    pub fn get_member_types(&self) -> &Vec<MemberType> {
        self.common.get_types2()
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
    pub fn new(name: &str, member_types: &Vec<MemberType>, names: &Vec<String>) -> Result<EnumDef,String> {
        Ok(EnumDef {
            common: StructEnumDef::new("enum",name,member_types,names)?
        })
    }

    pub fn name(&self) -> &str { &self.common.name() }

    pub fn get_branch_type(&self, name: &str) -> Option<MemberType> {
        self.common.type_from_name2(name)
    }
}

impl fmt::Debug for EnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.common)
    }
}
