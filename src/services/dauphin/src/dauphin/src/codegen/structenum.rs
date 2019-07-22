use std::collections::HashSet;
use std::fmt;

use crate::types::Type;

pub struct StructEnumDef {
    type_: String,
    name: String,
    types: Vec<Type>,
    names: Vec<String>
}

impl fmt::Debug for StructEnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{} {} {{ ",self.type_,self.name)?;
        for (i,t) in self.types.iter().enumerate() {
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
    pub fn new(type_: &str, name: &str, types: &Vec<Type>, names: &Vec<String>) -> Result<StructEnumDef,String> {
        no_duplicates(names)?;
        Ok(StructEnumDef {
            type_: type_.to_string(),
            name: name.to_string(),
            types: types.to_vec(),
            names: names.clone()
        })
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn get_names(&self) -> &Vec<String> { &self.names }
    pub fn get_types(&self) -> &Vec<Type> { &self.types }

    pub fn type_from_name(&self, name: &str) -> Option<&Type> {
        for (i,this_name) in self.names.iter().enumerate() {
            if this_name == name {
                return Some(&self.types[i]);
            }
        }
        None
    }
}

pub struct StructDef {
    common: StructEnumDef
}

impl StructDef {
    pub fn new(name: &str, types: &Vec<Type>, names: &Vec<String>) -> Result<StructDef,String> {
        Ok(StructDef {
            common: StructEnumDef::new("struct",name,types,names)?
        })
    }

    pub fn name(&self) -> &str { &self.common.name() }
    pub fn get_names(&self) -> &Vec<String> { &self.common.get_names() }

    pub fn get_member_types(&self) -> &Vec<Type> {
        self.common.get_types()
    }

    pub fn get_member_type(&self, name: &str) -> Option<&Type> {
        self.common.type_from_name(name)
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
    pub fn new(name: &str, types: &Vec<Type>, names: &Vec<String>) -> Result<EnumDef,String> {
        Ok(EnumDef {
            common: StructEnumDef::new("enum",name,types,names)?
        })
    }

    pub fn name(&self) -> &str { &self.common.name() }

    pub fn get_branch_type(&self, name: &str) -> Option<&Type> {
        self.common.type_from_name(name)
    }
}

impl fmt::Debug for EnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.common)
    }
}
