use std::fmt;

use crate::codegen::Register;

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash)]
pub enum BaseType {
    StringType,
    BytesType,
    NumberType,
    BooleanType,
    StructType(String),
    EnumType(String),
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
            BaseType::StructType(t) => t,
            BaseType::EnumType(t) => t
        };
        write!(f,"{}",v)?;
        Ok(())
    }
}

#[derive(PartialEq,Clone)]
pub enum Type {
    Base(BaseType),
    Vector(Box<Type>)
}

impl Type {
    pub fn to_typesigexpr(&self) -> TypeSigExpr {
        match self {
            Type::Base(v) => TypeSigExpr::Base(v.clone()),
            Type::Vector(v) => TypeSigExpr::Vector(Box::new(v.to_typesigexpr()))
        }
    }
}

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Base(t) => write!(f,"{:?}",t),
            Type::Vector(v) => write!(f,"vec({:?})",v)
        }
    }
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

    pub fn to_typepattern(&self) -> TypePattern {
        match self {
            TypeSigExpr::Base(t) => TypePattern::Base(t.clone()),
            TypeSigExpr::Vector(t) => TypePattern::Vector(Box::new(t.to_typepattern())),
            TypeSigExpr::Placeholder(_) => TypePattern::Any
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
pub enum TypePattern {
    Base(BaseType),
    Vector(Box<TypePattern>),
    Any
}

impl TypePattern {
    pub fn is_invalid(&self) -> bool {
        match self {
            TypePattern::Base(BaseType::Invalid) => true,
            TypePattern::Vector(v) => v.is_invalid(),
            _ => false
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

    pub fn to_typepattern(&self) -> TypePattern {
        match self {
            TypeSig::Right(x) => x.to_typepattern(),
            TypeSig::Left(x,_) => x.to_typepattern()
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
pub struct ArgumentType {
    writeonly: bool,
    sig: TypeSig
}

impl ArgumentType {
    pub fn new(typesig: &TypeSig) -> ArgumentType {
        ArgumentType {
            writeonly: false, sig: typesig.clone()
        }
    }

    pub fn new_writeonly(typesig: &TypeSig) -> ArgumentType {
        ArgumentType {
            writeonly: true, sig: typesig.clone()
        }
    }

    pub fn new_left(typesig: &TypeSigExpr, reg: &Register) -> ArgumentType {
        ArgumentType {
            writeonly: false, sig: TypeSig::Left(typesig.clone(),reg.clone())
        }
    }

    pub fn new_right(typesig: &TypeSigExpr) -> ArgumentType {
        ArgumentType {
            writeonly: false, sig: TypeSig::Right(typesig.clone())
        }
    }

    pub fn new_left_writeonly(typesig: &TypeSigExpr, reg: &Register) -> ArgumentType {
        ArgumentType {
            sig: TypeSig::Left(typesig.clone(),reg.clone()), writeonly: true
        }
    }

    pub fn new_right_writeonly(typesig: &TypeSigExpr) -> ArgumentType {
        ArgumentType {
            sig: TypeSig::Right(typesig.clone()), writeonly: true
        }
    }

    pub fn get_type(&self) -> &TypeSig { &self.sig }
    pub fn get_writeonly(&self) -> bool { self.writeonly }
}
