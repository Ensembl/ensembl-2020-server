use super::types::ArgumentType;
use crate::codegen::Register;

#[derive(Clone,Debug)]
pub struct ArgumentMatch {
    type_: ArgumentType,
    register: Register
}

impl ArgumentMatch {
    pub fn new(type_: &ArgumentType, register: &Register) -> ArgumentMatch {
        ArgumentMatch {
            type_: type_.clone(),
            register: register.clone()
        }
    }

    pub fn get_register(&self) -> &Register { &self.register }
    pub fn get_type(&self) -> &ArgumentType { &self.type_ }
}