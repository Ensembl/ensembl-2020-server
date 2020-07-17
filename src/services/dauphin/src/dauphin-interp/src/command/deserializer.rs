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

use crate::command::{ CommandDeserializer, CommandTypeId };

pub struct Deserializer {
    mapping: Vec<Box<dyn CommandDeserializer>>
}

impl Deserializer {
    pub fn new() -> Deserializer {
        Deserializer {
            mapping: vec![]
        }
    }

    pub fn add(&mut self, cd: Box<dyn CommandDeserializer>) -> Result<CommandTypeId,String> {
        let pos = self.mapping.len();
        self.mapping.push(cd);
        Ok(CommandTypeId(pos))
    }

    pub fn get(&self, cid: &CommandTypeId) -> Result<&Box<dyn CommandDeserializer>,String> {
        self.mapping.get(cid.0).ok_or_else(|| format!("No such command"))
    }
}