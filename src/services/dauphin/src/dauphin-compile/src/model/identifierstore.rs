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

use std::fmt::Debug;
use std::collections::HashMap;
use dauphin_interp::command::{ Identifier };

#[derive(Clone,Debug)]
pub struct IdentifierUse(pub Identifier,pub bool);

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct IdentifierPattern(pub Option<String>,pub String);

impl std::fmt::Display for IdentifierPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(module) = &self.0 {
            write!(f,"{}::{}",module,self.1)
        } else {
            write!(f,"{}",self.1)
        }
    }
}

#[derive(Debug)]
pub struct IdentifierStore<T> where T: Debug {
    store: HashMap<Identifier,T>
}

impl<T> IdentifierStore<T> where T: Debug {
    pub fn new() -> IdentifierStore<T> {
        IdentifierStore {
            store: HashMap::new()
        }
    }

    pub fn add(&mut self, identifier: &Identifier, value: T) {
        self.store.insert(identifier.clone(),value);
    }

    pub fn get_id(&self, identifier: &Identifier) -> Result<&T,String> {
        self.store.get(&identifier.clone()).ok_or_else(|| format!("No such identifier {}",identifier))
    }

    pub fn contains_key(&self, identifier: &Identifier) -> bool {
        self.store.get(&identifier.clone()).is_some()
    }
}
