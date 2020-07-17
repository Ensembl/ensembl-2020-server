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

use std::collections::{ HashMap, BTreeMap, HashSet };
use serde_cbor::Value as CborValue;
use crate::command::{ CommandSetId };

#[derive(Debug)]
pub struct OpcodeMapping {
    order: Vec<CommandSetId>,
    ready: bool,
    sid_next_offset: HashMap<CommandSetId,u32>,
    sid_base_opcode: HashMap<CommandSetId,u32>,
    opcode_sid: BTreeMap<u32,CommandSetId>,
    dont_serialize: HashSet<CommandSetId>
}

impl OpcodeMapping {
    pub fn new() -> OpcodeMapping {
        OpcodeMapping {
            order: vec![],
            ready: true,
            sid_next_offset: HashMap::new(),
            sid_base_opcode: HashMap::new(),
            opcode_sid: BTreeMap::new(),
            dont_serialize: HashSet::new()
        }
    }

    pub fn dont_serialize(&mut self, sid: &CommandSetId) {
        self.dont_serialize.insert(sid.clone());
    }

    pub fn recalculate(&mut self) {
        self.sid_base_opcode.clear();
        self.opcode_sid.clear();
        let mut high_water = 0;
        for sid in &self.order {
            let next_opcode = *self.sid_next_offset.get(sid).as_ref().unwrap();
            self.sid_base_opcode.insert(sid.clone(),high_water);
            self.opcode_sid.insert(high_water,sid.clone());
            high_water += next_opcode;
        }
        self.ready = true;
    }

    pub fn add_opcode(&mut self, sid: &CommandSetId, offset: u32) {
        if !self.sid_next_offset.contains_key(sid) {
            self.sid_next_offset.insert(sid.clone(),0);
            self.order.push(sid.clone());
        }
        let next = self.sid_next_offset.get_mut(&sid).unwrap();
        if *next <= offset { *next = offset+1; }
        self.ready = false;
    }

    pub fn serialize(&self) -> CborValue {
        let mut out = vec![];
        for sid in &self.order {
            if !self.dont_serialize.contains(sid) {
                let base_opcode = *self.sid_base_opcode.get(sid).as_ref().unwrap();
                out.push(CborValue::Integer(*base_opcode as i128));
                out.push(sid.serialize());
            }
        }
        CborValue::Array(out)
    }

    pub fn adjust(&mut self, mapping: &HashMap<CommandSetId,u32>) -> Result<(),String> {
        self.sid_base_opcode.clear();
        self.opcode_sid.clear();
        self.order.clear();
        for (sid,base) in mapping {
            self.order.push(sid.clone());
            self.sid_base_opcode.insert(sid.clone(),*base);
            self.opcode_sid.insert(*base,sid.clone());    
        }
        Ok(())
    }

    pub fn sid_to_offset(&self, sid: &CommandSetId) -> Result<u32,String> {
        if !self.ready { return Err(format!("recalculate not called after adding")); }
        self.sid_base_opcode.get(sid).map(|v| *v).ok_or_else(|| format!("no such sid"))
    }

    pub fn decode_opcode(&self, offset: u32) -> Result<(CommandSetId,u32),String> {
        if !self.ready { return Err(format!("recalculate not called after adding")); }
        if let Some((base,csi)) = self.opcode_sid.range(..(offset+1)).next_back() {
            Ok((csi.clone(),offset-base))
        } else {
            Err("no such offset".to_string())
        }
    }
}
