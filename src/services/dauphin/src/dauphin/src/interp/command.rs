use super::context::InterpContext;

pub trait Command {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String>;
}
