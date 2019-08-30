use super::types::{ RegisterType, InstructionConstraint, ExpressionType, BaseType };

pub struct TypeRule {

}

impl TypeRule {
    pub fn new() -> TypeRule {
        TypeRule {
            
        }
    }

    pub fn add(&mut self, sig: &InstructionConstraint) -> Result<(),String> {
        Ok(())
    }

    pub fn get_type(&self) -> Result<RegisterType,String> {
        Ok(RegisterType::NonReference(ExpressionType::Base(BaseType::Invalid)))
    }
}