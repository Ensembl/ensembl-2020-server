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

use std::collections::BTreeMap;
use serde_cbor::Value as CborValue;

#[derive(Debug,PartialEq)]
pub enum CborType {
    Integer,
    Bool,
    Text,
    Map,
    Tag,
    Array,
    Null,
    Float,
    Bytes
}

pub fn cbor_serialize(program: &CborValue) -> Result<Vec<u8>,String> {
    let mut buffer = Vec::new();
    serde_cbor::to_writer(&mut buffer,&program).map_err(|x| format!("{} while serialising",x))?;
    Ok(buffer)
}

pub fn cbor_make_map(keys: &[&str], mut values: Vec<CborValue>) -> Result<CborValue,String> {
    if keys.len() != values.len() {
        return Err("Bad cbor_make_map()".to_string());
    }
    let mut out = BTreeMap::new();
    for (i,v) in values.drain(..).enumerate() {
        out.insert(CborValue::Text(keys[i].to_string()),v);
    }
    Ok(CborValue::Map(out))
}

pub fn cbor_type(cbor: &CborValue, allowed: Option<&[CborType]>) -> Result<CborType,String> {
    let out = match cbor {
        CborValue::Integer(_) => CborType::Integer,
        CborValue::Bool(_) => CborType::Bool,
        CborValue::Text(_) => CborType::Text,
        CborValue::Map(_) => CborType::Map,
        CborValue::Tag(_,_) => CborType::Tag,
        CborValue::Array(_) => CborType::Array,
        CborValue::Null => CborType::Null,
        CborValue::Float(_) => CborType::Float,
        CborValue::Bytes(_) => CborType::Bytes,
        _ => { return Err(format!("unexpected cbort type (hidden")) }
    };
    if let Some(allowed) = allowed {
        if !allowed.contains(&out) {
            return Err(format!("unexpected cbor type: {:?}",out));
        }
    }
    Ok(out)
}

pub fn cbor_int(cbor: &CborValue, max: Option<i128>) -> Result<i128,String>  {
    match cbor {
        CborValue::Integer(x) => {
            if let Some(max) = max {
                if *x >= 0 && *x <= max { return Ok(*x); }
            } else {
                return Ok(*x);
            }
        },
        _ => {}
    }
    Err(format!("bad cbor: expected int, unexpected {:?}",cbor))
}

pub fn cbor_float(cbor: &CborValue) -> Result<f64,String>  {
    match cbor {
        CborValue::Float(x) => {
            return Ok(*x);
        },
        _ => {}
    }
    Err(format!("bad cbor: expected float, unexpected {:?}",cbor))
}

pub fn cbor_bool(cbor: &CborValue) -> Result<bool,String> {
    match cbor {
        CborValue::Bool(x) => Ok(*x),
        _ => Err(format!("bad cbor: expected bool, unexpected {:?}",cbor))
    }
}

pub fn cbor_string(cbor: &CborValue) -> Result<String,String> {
    match cbor {
        CborValue::Text(x) => Ok(x.to_string()),
        _ => Err(format!("bad cbor: expected string, unexpected {:?}",cbor))
    }
}

pub fn cbor_map<'a>(cbor: &'a CborValue, keys: &[&str]) -> Result<Vec<&'a CborValue>,String> {
    let mut out = vec![];
    match cbor {
        CborValue::Map(m) => {
            for key in keys {
                out.push(m.get(&CborValue::Text(key.to_string())).ok_or_else(|| format!("cbor: missing key {}",key))?);
            }
        },
        _ => { return Err(format!("bad cbor: expected map, unexpected {:?}",cbor)); }
    }
    Ok(out)
}

pub fn cbor_map_iter(cbor: &CborValue) -> Result<impl Iterator<Item=(&CborValue,&CborValue)>,String> {
    match cbor {
        CborValue::Map(m) => {
            Ok(m.iter())
        },
        _ => {
            return Err(format!("bad cbor: expected map, unexpected {:?}",cbor));
        }
    }
}

pub fn cbor_entry<'a>(cbor: &'a CborValue, key: &str) -> Result<Option<&'a CborValue>,String> {
    Ok(match cbor {
        CborValue::Map(m) => m.get(&CborValue::Text(key.to_string())),
        _ => { return Err(format!("bad cbor: expected map, unexpected {:?}",cbor)); }
    })
}

pub fn cbor_array<'a>(cbor: &'a CborValue, len: usize, or_more: bool) -> Result<&'a Vec<CborValue>,String> {
    match cbor {
        CborValue::Array(a) => {
            if a.len() == len || (a.len() >= len && or_more) {
                return Ok(a);
            }
        },
        _ => {}
    }
    Err(format!("bad cbor: expected array len={:?}/{:?}, unexpected {:?}",len,or_more,cbor))
}