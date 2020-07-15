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
use crate::common::CommandSetId;

pub struct CommandSetVerifier {
    seen: HashMap<(String,u32),String>,
}

impl CommandSetVerifier {
    pub fn new() -> CommandSetVerifier {
        CommandSetVerifier {
            seen: HashMap::new()
        }
    }

    pub fn register2(&mut self, set_id: &CommandSetId) -> Result<(),String> {
        let set_name = set_id.name().to_string();
        let set_major = set_id.version().0;
        if let Some(name) = self.seen.get(&(set_name.to_string(),set_major)) {
            return Err(format!("Attempt to register multiple versions {} and {}",set_id,name));
        }
        self.seen.insert((set_name.to_string(),set_major),set_id.to_string());
        Ok(())
    }
}
