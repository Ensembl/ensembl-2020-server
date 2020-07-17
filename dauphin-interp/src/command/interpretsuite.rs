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
use std::rc::Rc;
use serde_cbor::Value as CborValue;
use crate::command::{ CommandTypeId, CommandDeserializer, CommandSetId, Deserializer, InterpLibRegister, OpcodeMapping, CommandSetVerifier };
use crate::runtime::{ PayloadFactory };
use crate::util::cbor::{ cbor_array, cbor_int };

pub struct CommandInterpretSuite {
    store: Deserializer,
    offset_to_command: HashMap<(CommandSetId,u32),CommandTypeId>,
    opcode_mapper: OpcodeMapping,
    minors: HashMap<(String,u32),u32>,
    verifier: CommandSetVerifier,
    payloads: HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>
}

impl CommandInterpretSuite {
    pub fn new() -> CommandInterpretSuite {
        CommandInterpretSuite {
            opcode_mapper: OpcodeMapping::new(),
            offset_to_command: HashMap::new(),
            store: Deserializer::new(),
            minors: HashMap::new(),
            verifier: CommandSetVerifier::new(),
            payloads: HashMap::new()
        }
    }

    pub fn register(&mut self, mut set: InterpLibRegister) -> Result<(),String> {
        let sid = set.id().clone();
        let version = sid.version();
        self.minors.insert((sid.name().to_string(),version.0),version.1);
        for ds in set.drain_commands().drain(..) {
            if let Some((opcode,_)) = ds.get_opcode_len()? {
                let cid = self.store.add(ds)?;
                self.offset_to_command.insert((sid.clone(),opcode),cid.clone());
                self.opcode_mapper.add_opcode(&sid,opcode);
            }
        }
        for (k,p) in set.drain_payloads().drain() {
            self.payloads.insert(k,p);
        }
        self.verifier.register2(&sid)?;
        self.opcode_mapper.recalculate();
        Ok(())
    }

    pub fn copy_payloads(&self) -> HashMap<(String,String),Rc<Box<dyn PayloadFactory>>> {
        self.payloads.clone()
    }

    pub fn adjust(&mut self, cbor: &CborValue) -> Result<(),String> {
        let data = cbor_array(cbor,0,true)?;
        if data.len()%2 != 0 {
            return Err(format!("badly formed cbor"))
        }
        let mut adjustments = HashMap::new();
        for i in (0..data.len()).step_by(2) {
            let (sid,base) = (CommandSetId::deserialize(&data[i+1])?,cbor_int(&data[i],None)? as u32);
            let name = sid.name().to_string();
            let version = sid.version();
            if let Some(stored_minor) = self.minors.get(&(name.clone(),version.0)) {
                if *stored_minor < version.1 {
                    return Err(format!("version of {}.{} too old. have {} need {}",name,version.0,stored_minor,version.1));
                }
            } else {
                return Err(format!("missing command suite {}.{}",name,version.0));
            }
            adjustments.insert(sid,base);
        }
        self.opcode_mapper.adjust(&adjustments)?;
        Ok(())
    }

    pub fn get_deserializer(&self, real_opcode: u32) -> Result<&Box<dyn CommandDeserializer>,String> {
        let (sid,offset) = self.opcode_mapper.decode_opcode(real_opcode)?;
        let cid = self.offset_to_command.get(&(sid,offset)).ok_or(format!("Unknown opcode {}",real_opcode))?;
        self.store.get(cid)
    }
}
