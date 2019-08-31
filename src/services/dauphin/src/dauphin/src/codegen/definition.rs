use crate::types::{ Sig, TypeSigExpr };
use crate::typeinf::SignatureConstraint;

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

#[derive(Debug)]
pub struct ExprMacro {
    name: String
}

impl ExprMacro {
    pub fn new(name: &str) -> ExprMacro {
        ExprMacro { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

#[derive(Debug)]
pub struct StmtMacro {
    name: String
}

impl StmtMacro {
    pub fn new(name: &str) -> StmtMacro {
        StmtMacro { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

#[derive(Debug)]
pub struct FuncDecl {
    name: String,
    dst: TypeSigExpr,
    srcs: Vec<TypeSigExpr>,
    signature: SignatureConstraint
}

impl FuncDecl {
    pub fn new(name: &str, signature: &SignatureConstraint, dst: &TypeSigExpr, srcs: &Vec<TypeSigExpr>) -> FuncDecl {
        FuncDecl {
            name: name.to_string(),
            srcs: srcs.to_vec(),
            dst: dst.clone(),
            signature: signature.clone()
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn get_srcs(&self) -> &Vec<TypeSigExpr> { &self.srcs }
    pub fn get_dst(&self) -> &TypeSigExpr { &self.dst }
    pub fn get_signature(&self) -> &SignatureConstraint { &self.signature }
}

#[derive(Debug)]
pub struct ProcDecl {
    name: String,
    sigs: Vec<Sig>,
    signature: SignatureConstraint
}

impl ProcDecl {
    pub fn new(name: &str, sigs: &Vec<Sig>, signature: &SignatureConstraint) -> ProcDecl {
        ProcDecl { name: name.to_string(), sigs: sigs.to_vec(), signature: signature.clone() }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn sigs(&self) -> &Vec<Sig> { &self.sigs }
    pub fn get_signature(&self) -> &SignatureConstraint { &self.signature }
}
