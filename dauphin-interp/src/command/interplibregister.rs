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
use std::mem::replace;
use std::rc::Rc;
use crate::command::{ CommandDeserializer, CommandSetId };
use crate::runtime::PayloadFactory;

pub struct InterpLibRegister {
    id: CommandSetId,
    commands: Vec<Box<dyn CommandDeserializer + 'static>>,
    payloads: HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>
}

impl InterpLibRegister {
    pub fn new(id: &CommandSetId) -> InterpLibRegister {
        InterpLibRegister {
            id: id.clone(),
            commands: vec![],
            payloads: HashMap::new()
        }
    }

    pub fn id(&self) -> &CommandSetId { &self.id }

    pub fn push<F>(&mut self, deserializer: F) where F: CommandDeserializer + 'static {
        self.commands.push(Box::new(deserializer));
    }
    
    pub fn add_payload<P>(&mut self, set: &str, name: &str, pf: P) where P: PayloadFactory + 'static {
        self.payloads.insert((set.to_string(),name.to_string()),Rc::new(Box::new(pf)));
    }

    pub fn drain_commands(&mut self) -> Vec<Box<dyn CommandDeserializer>> {
        replace(&mut self.commands,vec![])
    }

    pub fn drain_payloads(&mut self) -> HashMap<(String,String),Rc<Box<dyn PayloadFactory>>> {
        replace(&mut self.payloads, HashMap::new())
    }
}
