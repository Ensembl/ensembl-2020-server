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

use crate::command::CommandType;
use dauphin_interp::command::CommandTypeId;

pub struct CommandTypeStore {
    commandtypes: Vec<Box<dyn CommandType>>
}

impl CommandTypeStore {
    pub fn new() -> CommandTypeStore {
        CommandTypeStore {
            commandtypes: vec![]
        }
    }

    pub fn add(&mut self, ct: Box<dyn CommandType>) -> CommandTypeId {
        let id = CommandTypeId(self.commandtypes.len());
        self.commandtypes.push(ct);
        id
    }

    pub fn get(&self, id: &CommandTypeId) -> &Box<dyn CommandType> {
        self.commandtypes.get(id.0).unwrap()
    }

    pub fn get_mut(&mut self, id: &CommandTypeId) -> &mut Box<dyn CommandType> {
        self.commandtypes.get_mut(id.0).unwrap()
    }
}