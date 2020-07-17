/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use super::types::{
    ArgumentExpressionConstraint, ArgumentConstraint, ExpressionType
};
use dauphin_interp::types::BaseType;

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
    pub(super) fn from_argumentexpressionconstraint<F>(aec: &ArgumentExpressionConstraint, mut cb: F) 
                -> ExpressionConstraint where F: FnMut(&str) -> usize {
        match aec {
            ArgumentExpressionConstraint::Base(b) => 
                ExpressionConstraint::Base(b.clone()),
            ArgumentExpressionConstraint::Vec(v) =>
                ExpressionConstraint::Vec(Box::new(ExpressionConstraint::from_argumentexpressionconstraint(v,cb))),
            ArgumentExpressionConstraint::Placeholder(s) =>
                ExpressionConstraint::Placeholder(Key::Internal(cb(&s)))
        }
    }

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
            ExpressionConstraint::Placeholder(_) => value.clone()
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
    pub(super) fn from_argumentconstraint<F>(ac: &ArgumentConstraint, cb: F)
                    -> TypeConstraint where F: FnMut(&str) -> usize {
        match ac {
            ArgumentConstraint::Reference(aec) =>
                TypeConstraint::Reference(ExpressionConstraint::from_argumentexpressionconstraint(aec,cb)),
            ArgumentConstraint::NonReference(aec) =>
                TypeConstraint::NonReference(ExpressionConstraint::from_argumentexpressionconstraint(aec,cb)),
        }
    }

    pub(super) fn get_expressionconstraint(&self) -> &ExpressionConstraint {
        match self {
            TypeConstraint::Reference(e) => e,
            TypeConstraint::NonReference(e) => e
        }
    }
}