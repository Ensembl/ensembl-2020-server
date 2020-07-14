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
use std::mem::replace;
use dauphin_interp_common::common::{ CommandSetId };
use dauphin_interp_common::interp::{ InterpLibRegister };
use super::command::{ CommandType };
use serde_cbor::Value as CborValue;
use crc::crc64::checksum_iso;

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
            print!("94 {:?} {:?}\n",base,csi);
            Ok((csi.clone(),offset-base))
        } else {
            Err("no such offset".to_string())
        }
    }
}

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

#[cfg(test)]
mod test {
    use std::rc::Rc;
    use std::cell::RefCell;
    use super::*;
    use crate::commands::{
        ConstCommandType, NumberConstCommandType, BooleanConstCommandType, StringConstCommandType
    };
    use crate::interp::harness::{ FakeDeserializer };
    use crate::interp::{ CompLibRegister, CommandCompileSuite, CommandTrigger, CommandInterpretSuite };
    use crate::generate::InstructionSuperType;
    use dauphin_interp_common::interp::{ InterpContext };

    #[test]
    fn test_command_smoke() {
        let csi = CommandSetId::new("test",(1,2),0x331BC4DCC0EC5896);
        let mut cs = CompLibRegister::new(&csi,None);
        cs.push("test1",Some(0),ConstCommandType::new());
        cs.push("test3",Some(2),NumberConstCommandType::new());
        cs.push("test2",Some(1),BooleanConstCommandType::new());
        cs.check_trace().expect("d");
        assert_eq!(&csi,cs.id());
    }

    #[test]
    fn test_command_get() {
        let v : Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        let csi = CommandSetId::new("test",(1,2),0x5F139093D228FB5B);
        let mut cs = InterpLibRegister::new(&csi);
        cs.push(FakeDeserializer(v.clone(),1));
        cs.push(FakeDeserializer(v.clone(),2));
        cs.push(FakeDeserializer(v.clone(),3));
        let mut cis = CommandInterpretSuite::new();
        cis.register(cs).expect("a");
        let mut context = InterpContext::new(&HashMap::new());
        cis.get_deserializer(1).expect("e").deserialize(0,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(1,*v.borrow());
        cis.get_deserializer(3).expect("e").deserialize(2,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(3,*v.borrow());
        assert!(cis.get_deserializer(4).is_err());
    }

    fn trace_type(cs: &mut CompLibRegister, name: &str, opcode: u32) {
        match opcode {
            0 => cs.push(name,Some(opcode),NumberConstCommandType::new()),
            1 => cs.push(name,Some(opcode),BooleanConstCommandType::new()),
            2 => cs.push(name,Some(opcode),ConstCommandType::new()),
            _ => cs.push(name,Some(opcode),StringConstCommandType::new())
        }
    }

    fn trace_check(trace: u64, cmds: Vec<(&str,u32)>) -> bool {
        let csi = CommandSetId::new("test",(1,2),trace);
        let mut cs = CompLibRegister::new(&csi,None);
        for (name,opcode) in &cmds {
            trace_type(&mut cs, name, *opcode);
        }
        cs.check_trace().is_ok()
    }

    #[test]
    fn verify_trace() {
        assert!(trace_check(0x331BC4DCC0EC5896,vec![("test1",0),("test2",1),("test3",2)]));
        assert!(trace_check(0x331BC4DCC0EC5896,vec![("test2",1),("test1",0),("test3",2)]));
        assert!(!trace_check(0x331BC4DCC0EC5896,vec![("test1",0),("test3",2)]));
        assert!(!trace_check(0x331BC4DCC0EC5896,vec![("test2",1),("test1",0),("test3",2),("test4",3)]));
    }

    #[test]
    fn duplicate_opcode_test() {
        let csi = CommandSetId::new("test",(1,2),0x331BC4DCC0EC5896);
        let mut cs = CompLibRegister::new(&csi,None);
        cs.push("test1",Some(0),ConstCommandType::new());
        cs.push("test2",Some(0),ConstCommandType::new());
        let mut ccs = CommandCompileSuite::new();
        ccs.register(cs).expect_err("a");
    }

    #[test]
    fn duplicate_name_test() {
        let csi = CommandSetId::new("test",(1,2),0x331BC4DCC0EC5896);
        let mut cs = CompLibRegister::new(&csi,None);
        cs.push("test1",Some(0),ConstCommandType::new());
        cs.push("test1",Some(1),ConstCommandType::new());
        let mut ccs = CommandCompileSuite::new();
        ccs.register(cs).expect_err("a");
    }

    #[test]
    fn test_mappings() {
        let csi = CommandSetId::new("test",(1,2),0x5CA3544A88CABB57);
        let mut cs = CompLibRegister::new(&csi,None);
        cs.push("test1",Some(0),ConstCommandType::new());
        cs.push("test2",Some(1),NumberConstCommandType::new());
        let mut ccs = CommandCompileSuite::new();
        ccs.register(cs).expect("c");
        assert_eq!(Some(0),ccs.get_opcode_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::Const)).expect("a"));
        assert_eq!(Some(1),ccs.get_opcode_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("b"));
    }
}