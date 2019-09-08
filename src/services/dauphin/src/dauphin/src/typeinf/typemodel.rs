use std::collections::HashMap;
use std::fmt;

use super::types::MemberType;
use crate::model::Register;

pub struct TypeModel {
    values: HashMap<Register,MemberType>
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
            values: HashMap::new()
        }
    }

    pub fn add(&mut self, reg: &Register, type_: &MemberType) {
        self.values.insert(reg.clone(),type_.clone());
    }

    pub fn get(&mut self, reg: &Register) -> Option<&MemberType> {
        self.values.get(reg)
    }

    pub fn remove(&mut self, reg: &Register) {
        self.values.remove(reg);
    }

    pub fn each_register(&self) -> impl Iterator<Item=(&Register,&MemberType)> {
        self.values.iter()
    }
}