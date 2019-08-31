use std::fmt;
use crate::model::Register;

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

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum ExpressionType {
    Base(BaseType),
    Vec(Box<ExpressionType>),
    Any
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash)]
pub enum MemberType {
    Base(BaseType),
    Vec(Box<MemberType>),
}

impl fmt::Debug for MemberType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemberType::Base(b) => write!(f,"{:?}",b)?,
            MemberType::Vec(b) => write!(f,"vec({:?})",b)?
        }
        Ok(())
    }
}

impl MemberType {
    pub fn to_argumentexpressionconstraint(&self) -> ArgumentExpressionConstraint {
        match self {
            MemberType::Base(b) => ArgumentExpressionConstraint::Base(b.clone()),
            MemberType::Vec(v) => ArgumentExpressionConstraint::Vec(
                Box::new(v.to_argumentexpressionconstraint()))
        }
    }

    pub fn to_expressiontype(&self) -> ExpressionType {
        match self {
            MemberType::Base(b) => ExpressionType::Base(b.clone()),
            MemberType::Vec(v) => ExpressionType::Vec(
                Box::new(v.to_expressiontype()))
        }
    }
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum ArgumentExpressionConstraint {
    Base(BaseType),
    Vec(Box<ArgumentExpressionConstraint>),
    Placeholder(String)
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum ArgumentConstraint {
    Reference(ArgumentExpressionConstraint),
    NonReference(ArgumentExpressionConstraint)
}

#[derive(Clone,Debug)]
pub struct InstructionConstraint {
    constraints: Vec<(ArgumentConstraint,Register)>
}

impl InstructionConstraint {
    pub fn new(members: &Vec<(ArgumentConstraint,Register)>) -> InstructionConstraint {
        InstructionConstraint {
            constraints: members.clone()
        }
    }

    pub fn each_member(&self) -> impl Iterator<Item=&(ArgumentConstraint,Register)> {
        self.constraints.iter()
    }
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum SignatureMemberConstraint {
    LValue(ArgumentExpressionConstraint),
    RValue(ArgumentExpressionConstraint)
}

impl SignatureMemberConstraint {
    pub fn to_argumentconstraint(&self) -> ArgumentConstraint {
        match self {
            SignatureMemberConstraint::LValue(v) => ArgumentConstraint::Reference(v.clone()),
            SignatureMemberConstraint::RValue(v) => ArgumentConstraint::NonReference(v.clone())
        }
    }
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub struct SignatureConstraint {
    constraints: Vec<SignatureMemberConstraint>
}

impl SignatureConstraint {
    pub fn new(members: &Vec<SignatureMemberConstraint>) -> SignatureConstraint {
        SignatureConstraint {
            constraints: members.clone()
        }
    }

    pub fn each_member(&self) -> impl Iterator<Item=&SignatureMemberConstraint> {
        self.constraints.iter()
    }
}
