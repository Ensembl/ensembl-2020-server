#[derive(Debug,PartialEq)]
pub enum Statement {
    Import(String),
    Error(String)
}