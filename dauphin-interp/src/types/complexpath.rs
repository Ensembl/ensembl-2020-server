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
use std::sync::{ Arc, Mutex };
use crate::command::Identifier;
use crate::util::cbor::{ cbor_array, cbor_string, cbor_int };
use serde_cbor::Value as CborValue;

lazy_static! {
    static ref NEXT_ANON: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
enum ComplexPathId {
    Named(Vec<(Identifier,String)>),
    Anon(u64)
}

impl ComplexPathId {
    pub fn new_anon() -> ComplexPathId {
        let mut next = NEXT_ANON.lock().unwrap();
        *next += 1;
        ComplexPathId::Anon(*next)
    }

    pub fn serialize(&self) -> Result<CborValue,String> {
        Ok(match self {
            ComplexPathId::Named(path) => {
                CborValue::Array(path.iter().map(|x| 
                    CborValue::Array(vec![x.0.serialize(),CborValue::Text(x.1.clone())])
                ).collect())
            },
            ComplexPathId::Anon(_) => 
                CborValue::Null
        })
    }

    pub fn deserialize(cbor: &CborValue) -> Result<ComplexPathId,String> {
        match cbor {
            CborValue::Array(data) => {
                Ok(ComplexPathId::Named(
                    data.iter().map::<Result<_,String>,_>(|x| {
                        let part = cbor_array(x,2,false)?;
                        Ok((Identifier::deserialize(&part[0])?,cbor_string(&part[1])?))
                }).collect::<Result<Vec<(_,_)>,_>>()?))
            },
            CborValue::Null => {
                Ok(ComplexPathId::new_anon())
            },
            _ => Err("bad path".to_string())
        }
    }
}

impl fmt::Display for ComplexPathId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComplexPathId::Named(path) if path.len() > 0 => {
                let path : Vec<String> = path.iter().map(|(id,field)| 
                    format!("{}:{}",id,field)
                ).collect();
                write!(f,"{}",path.join("."))
            },
            ComplexPathId::Named(_) => {
                write!(f,"*")
            },
            ComplexPathId::Anon(_) => {
                write!(f,"?")
            }
        }
    }
}


#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct ComplexPath {
    path: ComplexPathId,
    breaks: Vec<usize>
}

impl ComplexPath {
    pub fn new_empty() -> ComplexPath {
        ComplexPath {
            path: ComplexPathId::Named(vec![]),
            breaks: vec![]
        }
    }

    pub fn get_name(&self) -> Option<&[(Identifier,String)]> {
        match &self.path {
            ComplexPathId::Named(v) => Some(v),
            ComplexPathId::Anon(_) => None
        }
    }

    pub fn get_breaks(&self) -> &[usize] { &self.breaks }

    pub fn new_anon() -> ComplexPath {
        ComplexPath {
            path: ComplexPathId::new_anon(),
            breaks: vec![]
        }
    }

    pub fn add(&self, complex: &Identifier, field: &str) -> ComplexPath {
        let path = match &self.path {
            ComplexPathId::Named(name) => {
                let mut name = name.to_vec();
                name.push((complex.clone(),field.to_string()));
                ComplexPathId::Named(name)
            },
            ComplexPathId::Anon(_) => ComplexPathId::new_anon()
        };
        ComplexPath {
            path,
            breaks: self.breaks.to_vec()
        }
    }

    pub fn add_levels(&self, lev: usize) -> ComplexPath {
        let mut breaks = self.breaks.to_vec();
        breaks.push(lev);
        ComplexPath {
            path: self.path.clone(),
            breaks
        }        
    }

    pub fn serialize(&self) -> Result<CborValue,String> {
        let breaks = CborValue::Array(
            self.breaks.iter().map(|x| CborValue::Integer(*x as i128)).collect()
        );
        Ok(CborValue::Array(vec![self.path.serialize()?,breaks]))
    }

    pub fn deserialize(cbor: &CborValue) -> Result<ComplexPath,String> {
        let data = cbor_array(cbor,2,false)?;
        let breaks = cbor_array(&data[1],0,true)?.iter().map(|x| cbor_int(x,None).map(|x| x as usize)).collect::<Result<Vec<_>,_>>()?;
        let path = ComplexPathId::deserialize(&data[0])?;
        Ok(ComplexPath {
                path,
                breaks
        })
    }
}

impl fmt::Display for ComplexPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.path)
    }
}