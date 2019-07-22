use std::fmt;

use crate::codegen::Register;

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash)]
pub enum BaseType {
    StringType,
    BytesType,
    NumberType,
    BooleanType,
    IdentifiedType(String),
    Invalid
}
impl fmt::Debug for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = match self {
            BaseType::StringType => "string",
            BaseType::BytesType => "bytes",
            BaseType::NumberType => "number",
            BaseType::BooleanType => "boolean",
            BaseType::Invalid => "***INVALID***",
            BaseType::IdentifiedType(t) => t
        };
        write!(f,"{}",v)?;
        Ok(())
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Type {
    Base(BaseType),
    Vector(Box<Type>)
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash)]
pub enum TypeSigExpr {
    Base(BaseType),
    Vector(Box<TypeSigExpr>),
    Placeholder(String)
}

impl TypeSigExpr {
    pub fn get_placeholder(&self) -> Option<&str> {
        match self {
            TypeSigExpr::Base(_) => None,
            TypeSigExpr::Vector(t) => t.get_placeholder(),
            TypeSigExpr::Placeholder(p) => Some(p)
        }
    }

    pub fn is_invalid(&self) -> bool {
        match self {
            TypeSigExpr::Base(BaseType::Invalid) => true,
            TypeSigExpr::Base(_) => false,
            TypeSigExpr::Vector(t) => t.is_invalid(),
            TypeSigExpr::Placeholder(_) => false
        }
    }
}

impl fmt::Debug for TypeSigExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeSigExpr::Base(v) => write!(f,"{:?}",v),
            TypeSigExpr::Vector(v) => write!(f,"vec({:?})",v),
            TypeSigExpr::Placeholder(p) => write!(f,"{}",p)
        }
    }
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash)]
pub enum TypeSig {
    Right(TypeSigExpr),
    Left(TypeSigExpr,Register)
}

impl TypeSig {
    pub fn get_placeholder(&self) -> Option<&str> {
        self.expr().get_placeholder()
    }

    pub fn is_invalid(&self) -> bool {
        self.expr().is_invalid()
    }

    pub fn expr(&self) -> &TypeSigExpr {
        match self {
            TypeSig::Right(x) => x,
            TypeSig::Left(x,_) => x
        }
    }
}

impl fmt::Debug for TypeSig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeSig::Right(x) => write!(f,"{:?}",x),
            TypeSig::Left(x,r) => write!(f,"ref({:?},{:?})",x,r)
        }
    }
}

// TODO fix sig junk
#[derive(PartialEq, Debug, Clone)]
pub struct Sig {
    pub lvalue: Option<TypeSig>,
    pub out: bool,
    pub typesig: TypeSig
}

impl Sig {
    pub fn new_left_in(typesig: &TypeSigExpr, reg: &Register) -> Sig {
        Sig {
            lvalue: None, out: false,
            typesig: TypeSig::Left(typesig.clone(),reg.clone())
        }
    }

    pub fn new_right_in(typesig: &TypeSigExpr) -> Sig {
        Sig {
            lvalue: None, out: false,
            typesig: TypeSig::Right(typesig.clone())
        }
    }

    pub fn new_left_out(typesig: &TypeSigExpr, reg: &Register) -> Sig {
        Sig {
            lvalue: Some(TypeSig::Left(typesig.clone(),reg.clone())), out: true,
            typesig: TypeSig::Right(TypeSigExpr::Placeholder("_".to_string()))
        }
    }

    pub fn new_right_out(typesig: &TypeSigExpr) -> Sig {
        Sig {
            lvalue: Some(TypeSig::Right(typesig.clone())), out: true,
            typesig: TypeSig::Right(TypeSigExpr::Placeholder("_".to_string()))
        }
    }
}
