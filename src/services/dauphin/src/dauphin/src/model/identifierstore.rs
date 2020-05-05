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

#[derive(Clone,Debug,PartialEq,Eq,Hash,PartialOrd,Ord)]
pub struct Identifier(pub String,pub String,pub bool);

impl Identifier {
    pub fn to_pattern(&self) -> IdentifierPattern {
        IdentifierPattern(Some(self.0.clone()),self.1.clone())
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{}::{}",self.0,self.1)
    }
}

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
pub enum IdentifierStoreError {
    MultipleMatches(Vec<String>),
    NoMatch
}

#[derive(Debug)]
pub struct IdentifierStore<T> where T: Debug {
    uses: HashMap<String,Vec<String>>,
    store: HashMap<(String,String),T>
}

impl<T> IdentifierStore<T> where T: Debug {
    pub fn new() -> IdentifierStore<T> {
        IdentifierStore {
            uses: HashMap::new(),
            store: HashMap::new()
        }
    }

    pub fn add(&mut self, identifier: &Identifier, value: T) {
        self.store.insert((identifier.0.to_string(),identifier.1.to_string()),value);
        self.uses.entry(identifier.1.to_string()).or_insert(vec![]).push(identifier.0.to_string());
    }

    pub fn get_id(&self, identifier: &Identifier) -> Result<&T,IdentifierStoreError> {
        self.store.get(&(identifier.0.clone(),identifier.1.clone())).ok_or(IdentifierStoreError::NoMatch)
    }

    pub fn get(&self, pattern: &IdentifierPattern) -> Result<(String,&T),IdentifierStoreError> {
        if let Some(module) = &pattern.0 {
            self.store.get(&(module.to_string(),pattern.1.to_string()))
                .ok_or(IdentifierStoreError::NoMatch)
                .map(|v| (module.to_string(),v))
        } else if let Some(uses) = self.uses.get(&pattern.1) {
            if uses.len() > 1 {
                Err(IdentifierStoreError::MultipleMatches(uses.to_vec()))
            } else if uses.len() == 0 {
                Err(IdentifierStoreError::NoMatch) // should be impossible
            } else {
                self.store.get(&(uses[0].to_string(),pattern.1.to_string()))
                    .ok_or(IdentifierStoreError::NoMatch) // should be impossible
                    .map(|v| (uses[0].to_string(),v))
            }
        } else {
            Err(IdentifierStoreError::NoMatch)
        }
    }

    pub fn contains_key(&self, pattern: &IdentifierPattern) -> bool {
        self.get(pattern).is_ok()
    }
}

impl From<IdentifierStoreError> for String {
    fn from(from: IdentifierStoreError) -> String {
        match from {
            IdentifierStoreError::MultipleMatches(module) =>
                format!("Matches in multiple modules: {}",module.join(",")),
            IdentifierStoreError::NoMatch =>
                format!("Unknown Identifier")
        }

    }
}
