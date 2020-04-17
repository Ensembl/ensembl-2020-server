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

use std::fmt;
use crate::model::{ Register, cbor_int };
use serde_cbor::Value as CborValue;

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

impl BaseType {
    pub fn serialize(&self) -> Result<CborValue,String> {
        Ok(match self {
            BaseType::StringType => CborValue::Integer(0),
            BaseType::BytesType => CborValue::Integer(1),
            BaseType::NumberType => CborValue::Integer(2),
            BaseType::BooleanType => CborValue::Integer(3),
            _ => Err("cannot serialize complex basetypes")?
        })
    }

    pub fn deserialize(cbor: &CborValue) -> Result<BaseType,String> {
        Ok(match cbor_int(cbor,Some(3))? {
            0 => BaseType::StringType,
            1 => BaseType::BytesType,
            2 => BaseType::NumberType,
            3 => BaseType::BooleanType,
            _ => panic!("cbor_int failed")
        })
    }
}

impl fmt::Debug for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self)
    }
}

impl fmt::Display for BaseType {
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

impl ExpressionType {
    pub(super) fn to_membertype(&self, catchall: &BaseType) -> MemberType {
        match self {
            ExpressionType::Base(b) => MemberType::Base(b.clone()),
            ExpressionType::Vec(v) => MemberType::Vec(Box::new(v.to_membertype(catchall))),
            ExpressionType::Any => MemberType::Base(catchall.clone())
        }
    }
}

pub struct ContainerType(usize);

impl ContainerType {
    pub fn construct(&self, in_: MemberType) -> MemberType {
        let mut out = in_;
        for _ in 0..self.0 {
            out = MemberType::Vec(Box::new(out));
        }
        out
    }

    pub fn new_empty() -> ContainerType {
        ContainerType(0)
    }

    pub fn depth(&self) -> usize {
        self.0
    }

    pub fn merge(&self, other: &ContainerType) -> ContainerType {
        ContainerType(self.0+other.0)
    }
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

impl fmt::Display for MemberType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemberType::Base(b) => write!(f,"{}",b)?,
            MemberType::Vec(b) => write!(f,"vec({})",b)?
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

    pub fn get_base(&self) -> BaseType {
        match self {
            MemberType::Base(b) => b.clone(),
            MemberType::Vec(v) => v.get_base()
        }
    }

    pub fn get_container(&self) -> ContainerType {
        ContainerType(self.depth())
    }

    pub fn depth(&self) -> usize {
        match self {
            MemberType::Base(_) => 0,
            MemberType::Vec(v) => 1+v.depth()
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum MemberMode {
    RValue,
    LValue,
    FValue
}

impl MemberMode {
    pub fn deserialize(cbor: &CborValue) -> Result<MemberMode,String> {
        Ok(match cbor_int(cbor,Some(2))? {
            0 => MemberMode::RValue,
            1 => MemberMode::LValue,
            2 => MemberMode::FValue,
            _ => panic!("cbor_int failed")
        })
    }

    pub fn serialize(&self) -> CborValue {
        match self {
            MemberMode::RValue => CborValue::Integer(0),
            MemberMode::LValue => CborValue::Integer(1),
            MemberMode::FValue => CborValue::Integer(2)
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum MemberDataFlow {
    JustifiesCall,  /* If this register is justified, so is this call (ie out or in/out) */
    Normal          /* If this call is justified, so is this register (ie in) */
}

// XXX remove data-flow from sig
impl MemberDataFlow {
    pub fn deserialize(cbor: &CborValue) -> Result<MemberDataFlow,String> {
        Ok(match cbor_int(cbor,Some(2))? {
            0 => MemberDataFlow::JustifiesCall,
            1 => MemberDataFlow::Normal,
            _ => panic!("cbor_int failed")
        })
    }

    pub fn serialize(&self) -> CborValue {
        match self {
            MemberDataFlow::JustifiesCall => CborValue::Integer(0),
            MemberDataFlow::Normal => CborValue::Integer(1)
        }
    }
}

impl fmt::Display for MemberDataFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",match self {
            MemberDataFlow::JustifiesCall => "j",
            MemberDataFlow::Normal => "n"
        })
    }
}

impl fmt::Display for MemberMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",match self {
            MemberMode::RValue => "R",
            MemberMode::LValue => "L",
            MemberMode::FValue => "F"
        })
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
