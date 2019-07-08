use std::collections::{ HashMap, HashSet };
use std::fmt;

use crate::parser::{ Type, BaseType };
use super::definitionstore::DefStore;

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
    name: String
}

impl FuncDecl {
    pub fn new(name: &str) -> FuncDecl {
        FuncDecl { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

#[derive(Debug)]
pub struct ProcDecl {
    name: String
}

impl ProcDecl {
    pub fn new(name: &str) -> ProcDecl {
        ProcDecl { name: name.to_string() }
    }

    pub fn name(&self) -> &str { &self.name }
}

pub enum BaseTypeDef {
    StringType,
    BytesType,
    NumberType,
    BooleanType,
    StructType(String),
    EnumType(String)
}

impl BaseTypeDef {
    fn new(t: &BaseType, defstore: &DefStore) -> Result<BaseTypeDef,String> {
        Ok(match t {
            BaseType::StringType => BaseTypeDef::StringType,
            BaseType::BytesType => BaseTypeDef::BytesType,
            BaseType::NumberType => BaseTypeDef::NumberType,
            BaseType::BooleanType => BaseTypeDef::BooleanType,
            BaseType::IdentifiedType(name) => {
                if defstore.get_struct(name).is_some() {
                    BaseTypeDef::StructType(name.to_string())
                } else if defstore.get_enum(name).is_some() {
                    BaseTypeDef::EnumType(name.to_string())
                } else {
                    return Err(format!("No such struct/enum '{}'",name));
                }
            }
        })
    }
}

impl fmt::Debug for BaseTypeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseTypeDef::StringType => write!(f,"string")?,
            BaseTypeDef::BytesType => write!(f,"bytes")?,
            BaseTypeDef::NumberType => write!(f,"number")?,
            BaseTypeDef::BooleanType => write!(f,"boolean")?,
            BaseTypeDef::StructType(name) => write!(f,"{}",name)?,
            BaseTypeDef::EnumType(name) => write!(f,"{}",name)?,
        }
        Ok(())
    }
}

pub enum TypeDef {
    Base(BaseTypeDef),
    Vector(Box<TypeDef>)
}

impl TypeDef {
    fn new(t: &Type, defstore: &DefStore) -> Result<TypeDef,String> {
        Ok(match t {
            Type::Base(t) => TypeDef::Base(BaseTypeDef::new(t,defstore)?),
            Type::Vector(t) => TypeDef::Vector(Box::new(TypeDef::new(t,defstore)?))
        })
    }
}

impl fmt::Debug for TypeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeDef::Base(t) => write!(f,"{:?}",t),
            TypeDef::Vector(v) => write!(f,"vec({:?})",v)
        }
    }
}

pub struct StructEnumDef {
    type_: String,
    name: String,
    types: Vec<TypeDef>,
    names: HashMap<String,usize>
}

impl fmt::Debug for StructEnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let revname : HashMap<usize,String> = self.names.iter().map(|(a,b)| (*b,a.to_string())).collect();
        write!(f,"{} {} {{ ",self.type_,self.name)?;
        for (i,t) in self.types.iter().enumerate() {
            if i > 0 { write!(f,", ")?; }
            write!(f,"{}: ",revname[&i])?;
            write!(f,"{:?}",t)?;
        }
        write!(f," }}")?;
        Ok(())
    }
}

fn no_duplicates(input: &Vec<String>) -> Result<(),String> { // TODO test
    let mut seen = HashSet::new();
    for name in input {
        if seen.contains(name) {
            return Err(format!("Duplicate name: '{:?}'",name));
        }
        seen.insert(name.to_string());
    }
    Ok(())
}

fn collect_errors<T>(input: Vec<Result<T,String>>) -> Result<Vec<T>,String> {
    let mut outs = Vec::new();
    let mut errors = Vec::new();
    for t in input {
        if let Some(err) = t.as_ref().err() {
            errors.push(err.to_string());
        } else {
            outs.push(t.ok().unwrap());
        }
    }
    if errors.len() > 0 {
        let out : Vec<String> = errors.iter().map(|x| x.to_string()).collect();
        Err(out.join(", "))
    } else {
        Ok(outs)
    }
}

impl StructEnumDef {
    pub fn new(type_: &str, name: &str, types: &Vec<Type>, names: &Vec<String>, defstore: &DefStore) -> Result<StructEnumDef,String> {
        let types = collect_errors(types.iter().map(|x| TypeDef::new(x, defstore)).collect())?;
        no_duplicates(names)?;
        Ok(StructEnumDef {
            type_: type_.to_string(),
            name: name.to_string(),
            types,
            names: names.iter().enumerate().map(|(k,v)| (v.to_string(),k)).collect()
        })
    }

    pub fn name(&self) -> &str { &self.name }
}

pub struct StructDef {
    common: StructEnumDef
}

impl StructDef {
    pub fn new(name: &str, types: &Vec<Type>, names: &Vec<String>, defstore: &DefStore) -> Result<StructDef,String> {
        Ok(StructDef {
            common: StructEnumDef::new("struct",name,types,names,defstore)?
        })
    }

    pub fn name(&self) -> &str { &self.common.name() }
}

impl fmt::Debug for StructDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.common)
    }
}

pub struct EnumDef {
    common: StructEnumDef
}

impl EnumDef {
    pub fn new(name: &str, types: &Vec<Type>, names: &Vec<String>, defstore: &DefStore) -> Result<EnumDef,String> {
        Ok(EnumDef {
            common: StructEnumDef::new("enum",name,types,names,defstore)?
        })
    }

    pub fn name(&self) -> &str { &self.common.name() }
}

impl fmt::Debug for EnumDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.common)
    }
}
