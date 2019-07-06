use std::collections::HashMap;
use crate::parser::ParseError;

#[derive(Debug,PartialEq,Clone,Copy)]
pub enum InlineMode {
    LeftAssoc,
    RightAssoc,
    Prefix,
    Suffix
}

#[derive(Debug)]
pub struct Inline {
    symbol: String,
    name: String,
    statement: bool,
    precedence: f64,
    mode: InlineMode
}

impl Inline {
    pub fn new(symbol: &str, name: &str, statement: bool, precedence: f64, mode: &InlineMode) -> Inline {
        Inline {
            symbol: symbol.to_string(),
            name: name.to_string(),
            statement, precedence, mode: *mode
        }
    }

    pub fn symbol(&self) -> &str { &self.symbol }
    pub fn name(&self) -> &str { &self.name }
    pub fn precedence(&self) -> f64 { self.precedence }
    pub fn mode(&self) -> &InlineMode { &self.mode }
}

pub fn check_inline_symbol(sym: &str) -> Result<(),ParseError> {
    /* TODO valid */
    /* TODO not already present */
    Ok(())
}

pub struct ExprMacro {
    name: String
}

impl ExprMacro {
    pub fn new(name: &str) -> ExprMacro {
        ExprMacro { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

pub struct StmtMacro {
    name: String
}

impl StmtMacro {
    pub fn new(name: &str) -> StmtMacro {
        StmtMacro { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

pub struct FuncDecl {
    name: String
}

impl FuncDecl {
    pub fn new(name: &str) -> FuncDecl {
        FuncDecl { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

pub struct ProcDecl {
    name: String
}

impl ProcDecl {
    pub fn new(name: &str) -> ProcDecl {
        ProcDecl { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

pub struct StructDef {
    name: String
}

impl StructDef {
    pub fn new(name: &str) -> StructDef {
        StructDef { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

pub struct EnumDef {
    name: String
}

impl EnumDef {
    pub fn new(name: &str) -> EnumDef {
        EnumDef { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}
