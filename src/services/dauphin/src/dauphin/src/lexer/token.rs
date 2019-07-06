#[derive(Debug,PartialEq,Clone)]
pub enum Token {
    Identifier(String),
    Number(f64,String),
    Operator(String),
    Other(char),
    Error(String),
    LiteralString(String),
    LiteralBytes(Vec<u8>),
    EndOfFile,
    EndOfLex
}
