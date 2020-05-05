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

use std::collections::{ HashMap, HashSet };
use super::{ Identifier, IdentifierPattern };
use crate::lexer::Lexer;

pub struct IdentifierGuesser {
    uses: HashMap<String,HashSet<String>>
}

impl IdentifierGuesser {
    pub fn new() -> IdentifierGuesser {
        IdentifierGuesser {
            uses: HashMap::new()
        }
    }

    pub fn add(&mut self, lexer: &Lexer, pattern: &IdentifierPattern) -> Identifier {
        let module = pattern.0.as_ref().map(|x| x.to_string()).unwrap_or_else(|| lexer.get_module().to_string());
        if !self.uses.contains_key(&pattern.1) {
            self.uses.insert(pattern.1.clone(),HashSet::new());
        }
        self.uses.get_mut(&pattern.1).unwrap().insert(module.clone());
        Identifier(module,pattern.1.clone(),pattern.0.is_none())
    }

    pub fn guess(&mut self, lexer: &Lexer, pattern: &IdentifierPattern) -> Result<Identifier,String> {
        if let Some(module) = &pattern.0 {
            return Ok(Identifier(module.clone(),pattern.1.clone(),false));
        }
        if let Some(modules) = self.uses.get(&pattern.1) {
            if modules.len() == 1 {
                let only = modules.iter().next().unwrap();
                return Ok(Identifier(only.clone(),pattern.1.clone(),true));
            } else if modules.contains(lexer.get_module()) {
                return Ok(Identifier(lexer.get_module().to_string(),pattern.1.clone(),true));
            } else {
                return Err(format!("Multiple matches for unqualified identifier {} {}",pattern.1,modules.iter().cloned().collect::<Vec<_>>().join(", ")));
            }
        } else {
            return Ok(Identifier(lexer.get_module().to_string(),pattern.1.clone(),true));
        }
    }
}
