use std::fmt;
use hex;

use crate::codegen::InlineMode;
use crate::lexer::Lexer;

#[derive(PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    LiteralString(String),
    LiteralBytes(Vec<u8>),
    LiteralBool(bool),
    Operator(String,Vec<Expression>),
    Star(Box<Expression>),
    Square(Box<Expression>),
    Bracket(Box<Expression>,Box<Expression>),
    Filter(Box<Expression>,Box<Expression>),
    Dot(Box<Expression>,String),
    Query(Box<Expression>,String),
    Pling(Box<Expression>,String),
    Vector(Vec<Expression>),
    CtorStruct(String,Vec<Expression>,Vec<String>),
    CtorEnum(String,String,Box<Expression>),
    Dollar,
    At
}

fn write_csl(f: &mut fmt::Formatter<'_>, exprs: &Vec<Expression>) -> fmt::Result {
    for (i,sub) in exprs.iter().enumerate() {
        if i > 0 {
            write!(f,",")?;
        }
        write!(f,"{:?}",sub)?;
    }
    Ok(())
}

fn write_csl_named(f: &mut fmt::Formatter<'_>, exprs: &Vec<Expression>, names: &Vec<String>) -> fmt::Result {
    let mut names = names.iter();
    for (i,sub) in exprs.iter().enumerate() {
        let name = names.next().unwrap();
        if i > 0 {
            write!(f,",")?;
        }
        write!(f,"{}: {:?}",name,sub)?;
    }
    Ok(())
}

impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Identifier(id) => write!(f,"{}",id),
            Expression::Number(n) => write!(f,"{}",n),
            Expression::LiteralString(s) => write!(f,"{:?}",s),
            Expression::LiteralBytes(b) => write!(f,"'{}'",hex::encode(b)),
            Expression::LiteralBool(b) => write!(f,"{}",if *b { "true" } else {"false"}),
            Expression::Star(s) => write!(f,"*({:?})",s),
            Expression::Square(s) => write!(f,"({:?})[]",s),
            Expression::Bracket(left,inner) => write!(f,"({:?})[{:?}]",left,inner),
            Expression::Filter(left,inner) => write!(f,"({:?})&[{:?}]",left,inner),
            Expression::Dot(expr,key) => write!(f,"{:?}.{}",expr,key),
            Expression::Query(expr,key) => write!(f,"{:?}?{}",expr,key),
            Expression::Pling(expr,key) => write!(f,"{:?}!{}",expr,key),
            Expression::Dollar => write!(f,"$"),
            Expression::At => write!(f,"@"),
            Expression::Vector(x) => {
                write!(f,"[")?;
                write_csl(f,x)?;
                write!(f,"]")?;
                Ok(())
            }
            Expression::Operator(s,x) => {
                write!(f,"{}(",s)?;
                write_csl(f,x)?;
                write!(f,")")?;
                Ok(())
            },
            Expression::CtorStruct(s,x,n) => {
                write!(f,"{} {{",s)?;
                write_csl_named(f,x,n)?;
                write!(f,"}}")?;
                Ok(())
            },
            Expression::CtorEnum(e,b,v) => {
                write!(f,"{}:{} {:?}",e,b,v)?;
                Ok(())
            }
        }
    }
}

/* TODO economical statement line numbers */
#[derive(PartialEq)]
pub struct Statement(pub String,pub Vec<Expression>,pub String,pub u32);

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}(",self.0)?;
        for (i,sub) in self.1.iter().enumerate() {
            if i > 0 {
                write!(f,",")?;
            }
            write!(f,"{:?}",sub)?;
        }
        write!(f,")")?;
        Ok(())
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum BaseType {
    StringType,
    BytesType,
    NumberType,
    BooleanType,
    IdentifiedType(String)
}

#[derive(PartialEq, Debug, Clone)]
pub enum Type {
    Base(BaseType),
    Vector(Box<Type>)
}

#[derive(PartialEq, Debug, Clone)]
pub enum TypeSig {
    Base(BaseType),
    Vector(Box<Type>),
    Placeholder(String)
}

#[derive(PartialEq, Debug, Clone)]
pub struct Sig {
    pub lvalue: bool,
    pub typesig: TypeSig
}

#[derive(Debug,PartialEq)]
pub enum ParserStatement {
    Import(String),
    Inline(String,String,InlineMode,f64),
    ExprMacro(String),
    StmtMacro(String),
    FuncDecl(String),
    ProcDecl(String,Vec<Sig>),
    Regular(Statement),
    StructDef(String,Vec<Type>,Vec<String>),
    EnumDef(String,Vec<Type>,Vec<String>),
    EndOfParse
}

#[derive(Debug,PartialEq)]
pub struct ParseError {
    error: String
}

impl ParseError {
    pub fn new(error: &str, lexer: &Lexer) -> ParseError {
        let (file,line,col) = lexer.position();
        ParseError {
            error: format!("{} at line {} column {} in {}",error,line,col,file)
        }
    }

    pub fn message(&self) -> &str { &self.error }
}