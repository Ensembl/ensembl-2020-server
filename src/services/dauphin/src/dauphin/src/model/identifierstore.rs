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

use std::collections::HashMap;

pub enum IdentifierStoreError {
    MultipleMatches(Vec<String>),
    NoMatch
}

#[derive(Debug)]
pub struct IdentifierStore<T> {
    uses: HashMap<String,Vec<String>>,
    store: HashMap<(String,String),T>
}

impl<T> IdentifierStore<T> {
    pub fn new() -> IdentifierStore<T> {
        IdentifierStore {
            uses: HashMap::new(),
            store: HashMap::new()
        }
    }

    pub fn add(&mut self, module: &str, name: &str, value: T) {
        self.store.insert((module.to_string(),name.to_string()),value);
        self.uses.entry(name.to_string()).or_insert(vec![]).push(module.to_string());
    }

    pub fn get(&self, module: Option<&str>, name: &str) -> Result<(String,&T),IdentifierStoreError> {
        if let Some(module) = module {
            self.store.get(&(module.to_string(),name.to_string()))
                .ok_or(IdentifierStoreError::NoMatch)
                .map(|v| (module.to_string(),v))
        } else if let Some(uses) = self.uses.get(name) {
            if uses.len() > 1 {
                Err(IdentifierStoreError::MultipleMatches(uses.to_vec()))
            } else if uses.len() == 0 {
                Err(IdentifierStoreError::NoMatch) // should be impossible
            } else {
                self.store.get(&(uses[0].to_string(),name.to_string()))
                    .ok_or(IdentifierStoreError::NoMatch) // should be impossible
                    .map(|v| (uses[0].to_string(),v))
            }
        } else {
            Err(IdentifierStoreError::NoMatch)
        }
    }

    pub fn contains_key(&self, module: Option<&str>, name: &str) -> bool {
        self.get(module,name).is_ok()
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
