use super::types::{ BaseType, ExpressionType, RegisterType };

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub(super) enum Key {
    Internal(usize),
    External(usize)
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub(super) enum ExpressionConstraint {
    Base(BaseType),
    Vec(Box<ExpressionConstraint>),
    Placeholder(Key)
}

impl ExpressionConstraint {
    pub(super) fn get_placeholder(&self) -> Option<&Key> {
        match self {
            ExpressionConstraint::Base(_) => None,
            ExpressionConstraint::Vec(v) => v.get_placeholder(),
            ExpressionConstraint::Placeholder(k) => Some(k)
        }
    }

    pub(super) fn substitute(&self, value: &ExpressionConstraint) -> ExpressionConstraint {
        match self {
            ExpressionConstraint::Base(b) => ExpressionConstraint::Base(b.clone()),
            ExpressionConstraint::Vec(v) => ExpressionConstraint::Vec(Box::new(v.substitute(value))),
            ExpressionConstraint::Placeholder(k) => value.clone()
        }
    }

    pub(super) fn to_expressiontype(&self) -> ExpressionType {
        match self {
            ExpressionConstraint::Base(b) => ExpressionType::Base(b.clone()),
            ExpressionConstraint::Vec(v) => ExpressionType::Vec(Box::new(v.to_expressiontype())),
            ExpressionConstraint::Placeholder(_) => ExpressionType::Any
        }
    }
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash,Debug)]
pub(super) enum TypeConstraint {
    Reference(ExpressionConstraint),
    NonReference(ExpressionConstraint)
}

impl TypeConstraint {
    pub(super) fn get_expressionconstraint(&self) -> &ExpressionConstraint {
        match self {
            TypeConstraint::Reference(e) => e,
            TypeConstraint::NonReference(e) => e
        }
    }

    pub(super) fn is_reference(&self) -> bool {
        match self {
            TypeConstraint::Reference(_) => true,
            TypeConstraint::NonReference(_) => false
        }
    }

    pub(super) fn get_placeholder(&self) -> Option<&Key> {
        self.get_expressionconstraint().get_placeholder()
    }

    pub(super) fn substitute(&self, value: &ExpressionConstraint) -> TypeConstraint {
        match self {
            TypeConstraint::Reference(e) => TypeConstraint::Reference(e.substitute(value)),
            TypeConstraint::NonReference(e) => TypeConstraint::NonReference(e.substitute(value))
        }        
    }

    pub(super) fn to_registertype(&self) -> RegisterType {
        match self {
            TypeConstraint::Reference(e) => RegisterType::Reference(e.to_expressiontype()),
            TypeConstraint::NonReference(e) => RegisterType::NonReference(e.to_expressiontype())
        }
    }
}