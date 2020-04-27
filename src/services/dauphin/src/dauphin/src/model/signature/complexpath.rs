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
use crate::model::{ cbor_array, cbor_string, cbor_type, CborType };
use serde_cbor::Value as CborValue;

lazy_static! {
    static ref NEXT_ANON: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
enum ComplexPathName {
    Named(Vec<String>),
    Anon(u64)
}

impl ComplexPathName {
    pub fn serialize(&self) -> Result<CborValue,String> {
        match self {
            ComplexPathName::Named(name) =>
                Ok(CborValue::Array(name.iter().map(|x| CborValue::Text(x.to_string())).collect())),
            ComplexPathName::Anon(_) =>
                Ok(CborValue::Null)
        }
    }
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct ComplexPath {
    path: ComplexPathName
}

impl ComplexPath {
    pub fn new_empty() -> ComplexPath {
        ComplexPath {
            path: ComplexPathName::Named(vec![])
        }
    }

    fn anon_pathname() -> ComplexPathName {
        let mut next = NEXT_ANON.lock().unwrap();
        *next += 1;
        ComplexPathName::Anon(*next)
    }

    pub fn new_anon() -> ComplexPath {
        ComplexPath {
            path: ComplexPath::anon_pathname()
        }
    }

    pub fn add(&self, added: &str) -> ComplexPath {
        let path = match &self.path {
            ComplexPathName::Named(name) => {
                let mut name = name.to_vec();
                name.push(added.to_string());
                ComplexPathName::Named(name)
            },
            ComplexPathName::Anon(_) => ComplexPath::anon_pathname()
        };
        ComplexPath {
            path
        }
    }

    pub fn serialize(&self) -> Result<CborValue,String> {
        Ok(self.path.serialize()?)
    }

    pub fn deserialize(cbor: &CborValue) -> Result<ComplexPath,String> {
        match cbor_type(cbor,Some(&vec![CborType::Array,CborType::Null]))? {
            CborType::Array => {
                let path = cbor_array(cbor,0,true)?.iter().map(|x| cbor_string(x)).collect::<Result<Vec<_>,_>>()?;
                Ok(ComplexPath {
                    path: ComplexPathName::Named(path)
                })
            },
            CborType::Null => {
                Ok(ComplexPath::new_anon())
            },
            _ => panic!("cbor_type invariant failed")
        }
    }
}

impl fmt::Display for ComplexPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            ComplexPathName::Named(name) if name.len() > 0 => {
                write!(f,"{}",name.iter().map(|x| format!("{}",x)).collect::<Vec<_>>().join("."))
            },
            ComplexPathName::Named(_) => { write!(f,"*") },
            ComplexPathName::Anon(_) => { write!(f,"?") }
        }
    }
}