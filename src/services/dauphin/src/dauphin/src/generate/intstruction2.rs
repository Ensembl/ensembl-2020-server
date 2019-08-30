use crate::codegen::Register2;

#[derive(Debug)]
pub enum Instruction2 {
    Proc(String,Vec<Register2>),
    NumberConst(Register2,f64),
    BooleanConst(Register2,bool),
    StringConst(Register2,String),
    BytesConst(Register2,Vec<u8>),
    List(Register2),
    Push(Register2,Register2),
}