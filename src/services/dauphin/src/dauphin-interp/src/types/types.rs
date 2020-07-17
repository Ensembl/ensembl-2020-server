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
use crate::util::cbor::cbor_int;
use crate::command::Identifier;
use serde_cbor::Value as CborValue;

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)]
pub enum MemberMode {
    In,
    InOut,
    Filter,
    Out
}

impl MemberMode {
    pub fn deserialize(cbor: &CborValue) -> Result<MemberMode,String> {
        Ok(match cbor_int(cbor,Some(3))? {
            0 => MemberMode::In,
            1 => MemberMode::InOut,
            2 => MemberMode::Filter,
            3 => MemberMode::Out,
            _ => panic!("cbor_int failed")
        })
    }

    pub fn serialize(&self) -> CborValue {
        match self {
            MemberMode::In => CborValue::Integer(0),
            MemberMode::InOut => CborValue::Integer(1),
            MemberMode::Filter => CborValue::Integer(2),
            MemberMode::Out => CborValue::Integer(3),
        }
    }
}

impl fmt::Display for MemberMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",match self {
            MemberMode::In => "R",
            MemberMode::InOut => "L",
            MemberMode::Filter => "F",
            MemberMode::Out => "Z"
        })
    }
}

#[derive(PartialEq,Eq,Clone,PartialOrd,Ord,Hash)]
pub enum BaseType {
    StringType,
    BytesType,
    NumberType,
    BooleanType,
    StructType(Identifier),
    EnumType(Identifier),
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
            BaseType::StringType => "string".to_string(),
            BaseType::BytesType => "bytes".to_string(),
            BaseType::NumberType => "number".to_string(),
            BaseType::BooleanType => "boolean".to_string(),
            BaseType::Invalid => "***INVALID***".to_string(),
            BaseType::StructType(t) => t.to_string(),
            BaseType::EnumType(t) => t.to_string()
        };
        write!(f,"{}",v)?;
        Ok(())
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)]
pub enum MemberDataFlow { In, Out, InOut }

// XXX remove data-flow from sig
impl MemberDataFlow {
    pub fn deserialize(cbor: &CborValue) -> Result<MemberDataFlow,String> {
        Ok(match cbor_int(cbor,Some(2))? {
            0 => MemberDataFlow::In,
            1 => MemberDataFlow::Out,
            2 => MemberDataFlow::InOut,
            _ => panic!("cbor_int failed")
        })
    }

    pub fn serialize(&self) -> CborValue {
        match self {
            MemberDataFlow::In => CborValue::Integer(0),
            MemberDataFlow::Out => CborValue::Integer(1),
            MemberDataFlow::InOut => CborValue::Integer(2)
        }
    }
}

impl fmt::Display for MemberDataFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",match self {
            MemberDataFlow::In => "i",
            MemberDataFlow::Out => "o",
            MemberDataFlow::InOut => "io",
        })
    }
}
