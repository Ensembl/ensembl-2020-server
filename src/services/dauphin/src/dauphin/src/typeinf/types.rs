#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum BaseType {
    StringType,
    BytesType,
    NumberType,
    BooleanType,
    StructType(String),
    EnumType(String),
    Invalid
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum ExpressionType {
    Base(BaseType),
    Vec(Box<ExpressionType>),
    Any
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum RegisterType {
    Reference(ExpressionType),
    NonReference(ExpressionType)
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
    constraints: Vec<ArgumentConstraint>
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub enum SignatureMemberConstraint {
    LValue(ArgumentExpressionConstraint),
    RValue(ArgumentExpressionConstraint)
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
