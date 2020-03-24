use std::collections::BTreeMap;
use std::fmt;

use super::types::MemberType;
use crate::model::Register;

pub struct TypeModel {
    values: BTreeMap<Register,MemberType>
}

impl fmt::Debug for TypeModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut keys : Vec<Register> = self.values.keys().cloned().collect();
        keys.sort();
        for reg in &keys {
            write!(f,"{:?} : {:?}\n",reg,self.values[reg])?;
        }
        Ok(())
    }
}

impl TypeModel {
    pub fn new() -> TypeModel {
        TypeModel {
            values: BTreeMap::new()
        }
    }

    pub fn add(&mut self, reg: &Register, type_: &MemberType) {
        self.values.insert(reg.clone(),type_.clone());
    }

    pub fn get(&mut self, reg: &Register) -> Option<&MemberType> {
        self.values.get(reg)
    }

    pub fn each_register(&self) -> impl Iterator<Item=(&Register,&MemberType)> {
        self.values.iter()
    }
}