use super::super::context::{InterpContext };
use super::super::value::InterpValueData;
use super::super::command::Command;
use crate::model::Register;

pub struct NilCommand(pub(crate) Register);

impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.write_empty(&self.0);
        Ok(())
    }
}

// XXX commit!

pub struct NumberConstCommand(pub(crate) Register,pub(crate) f64);

impl Command for NumberConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.write_numbers(&self.0)?.borrow_mut().push(self.1);
        Ok(())
    }
}

pub struct ConstCommand(pub(crate) Register,pub(crate) Vec<f64>);

impl Command for ConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.set_numbers(&self.0,self.1.to_vec())?;
        Ok(())
    }
}
