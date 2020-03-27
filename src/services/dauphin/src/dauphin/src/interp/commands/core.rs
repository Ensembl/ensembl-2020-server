use super::super::context::{ InterpContext };
use super::super::value::InterpValue;
use super::super::command::Command;
use crate::model::Register;

pub struct NilCommand(pub(crate) Register);

impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) {
        context.insert(&self.0,InterpValue::Empty);
    }
}

pub struct NumberConstCommand(pub(crate) Register,pub(crate) f64);

impl Command for NumberConstCommand {
    fn execute(&self, context: &mut InterpContext) {
        context.insert(&self.0,InterpValue::Numbers(vec![self.1]));
    }
}

pub struct ConstCommand(pub(crate) Register,pub(crate) Vec<f64>);

impl Command for ConstCommand {
    fn execute(&self, context: &mut InterpContext) {
        context.insert(&self.0,InterpValue::Numbers(self.1.iter().map(|x| *x).collect()));
    }
}
