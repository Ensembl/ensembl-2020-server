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
use dauphin_interp::command::{ CommandSetId, InterpLibRegister };
use crate::command::CommandType;
use serde_cbor::Value as CborValue;
use crc::crc64::checksum_iso;

pub struct CompLibRegister {
    trace: HashMap<u32,(String,usize)>,
    id: CommandSetId,
    interp_lib_register: Option<InterpLibRegister>,
    commands: Vec<(Option<u32>,Box<dyn CommandType + 'static>)>,
    headers: Vec<(String,String)>,
    dynamic_data: Vec<Vec<u8>>
}

impl CompLibRegister {
    pub fn new(id: &CommandSetId, interp_lib_register: Option<InterpLibRegister>) -> CompLibRegister {
        CompLibRegister {
            trace: HashMap::new(),
            id: id.clone(),
            interp_lib_register,
            commands: vec![],
            headers: vec![],
            dynamic_data: vec![],
        }
    }

    pub fn id(&self) -> &CommandSetId { &self.id }

    pub fn push<T>(&mut self, name: &str, offset: Option<u32>, commandtype: T)
                where T: CommandType + 'static {
        if let Some(offset) = offset {
            let sch = commandtype.get_schema();
            self.trace.insert(offset,(name.to_string(),sch.values));
        }
        self.commands.push((offset,Box::new(commandtype)));
    }

    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(),value.to_string()));
    }

    pub fn dynamic_data(&mut self, data: &[u8]) {
        self.dynamic_data.push(data.to_vec())
    }
    
    pub(super) fn drain_interp_lib_register(&mut self) -> Option<InterpLibRegister> {
        replace(&mut self.interp_lib_register,None)
    }

    pub(super) fn drain_commands(&mut self) -> Vec<(Option<u32>,Box<dyn CommandType>)> {
        replace(&mut self.commands,vec![])
    }

    pub(super) fn drain_headers(&mut self) -> Vec<(String,String)> {
        replace(&mut self.headers,vec![])
    }

    pub(super) fn drain_dynamic_data(&mut self) -> Vec<Vec<u8>> {
        replace(&mut self.dynamic_data,vec![])
    }

    fn cbor_trace(&self) -> Result<CborValue,String> {
        let mut items : HashMap<u32,CborValue> = self.trace.iter()
            .map(|(k,v)| (*k,CborValue::Array(vec![
                CborValue::Integer(*k as i128),
                CborValue::Text(v.0.to_string()),
                CborValue::Integer(v.1 as i128)
            ])))
            .collect();
        let mut keys : Vec<u32> = items.keys().cloned().collect();
        keys.sort();
        let mut out = vec![];
        for key in keys.iter() {
            out.push(items.remove(key).ok_or(format!("internal error tracing"))?);
        }
        Ok(CborValue::Array(out))
    }

    pub(super) fn check_trace(&self) -> Result<(),String> {
        let got = checksum_iso(&serde_cbor::to_vec(&self.cbor_trace()?).map_err(|_| format!("tracing failed"))?);
        if got != self.id.trace() {
            Err(format!("trace comparison failed for {}: expected {:08X}, got {:08X}",self.id(),self.id.trace(),got))
        } else {
            Ok(())
        }
    }
}
