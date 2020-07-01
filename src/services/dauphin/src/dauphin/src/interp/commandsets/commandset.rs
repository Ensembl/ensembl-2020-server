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

use crate::cli::Config;
use std::collections::{ HashMap, HashSet, BTreeMap }; // hashbrown
use std::rc::Rc;
use super::CommandSetId;
use super::command::{ CommandType, CommandTrigger };
use crate::interp::{ PayloadFactory, CompilerLink };
use crate::model::cbor_map_iter;
use serde_cbor::Value as CborValue;
use crc::crc64::checksum_iso;

pub struct CommandSet {
    names: HashSet<String>,
    headers: HashMap<String,String>,
    trace: HashMap<u32,String>,
    csi: CommandSetId,
    commands: HashMap<u32,Box<dyn CommandType>>,
    mapping: HashMap<CommandTrigger,u32>,
    compile_only: bool,
    payloads: HashMap<String,Rc<Box<dyn PayloadFactory>>>
}

impl CommandSet {
    pub fn new(csi: &CommandSetId, compile_only: bool) -> CommandSet {
        CommandSet {
            headers: HashMap::new(),
            names: HashSet::new(),
            trace: HashMap::new(),
            csi: csi.clone(),
            commands: HashMap::new(),
            mapping: HashMap::new(),
            compile_only,
            payloads: HashMap::new()
        }
    }

    pub(super) fn id(&self) -> &CommandSetId { &self.csi }
    pub(super) fn get_mappings(&mut self) -> &HashMap<CommandTrigger,u32> {
        &self.mapping
    }
    pub(super) fn get(&self, opcode: u32) -> Result<&Box<dyn CommandType>,String> {
        self.commands.get(&opcode).ok_or_else(|| format!("No such opcode {}",opcode))
    }
    pub(super) fn compile_only(&self) -> bool { self.compile_only }

    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(),value.to_string());
    }

    pub fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let mut out = BTreeMap::new();
        for (trigger,id) in self.mapping.iter() {
            if config.get_verbose() > 1 {
                print!("dynamic data for {}\n",trigger);
            }
            let command = self.commands.get(id).ok_or_else(|| format!("internal inconsistency for id {}",id))?;
            out.insert(trigger.serialize(),command.generate_dynamic_data(linker,config)?);
        }
        Ok(CborValue::Map(out))
    }

    pub fn load_dynamic_data(&mut self, data: &[u8]) -> Result<(),String> {
        let data : CborValue = serde_cbor::from_slice(&data).map_err(|x| format!("{} while deserialising {:?}",x,self.csi))?;
        for (trigger,data) in cbor_map_iter(&data)? {
            let trigger = CommandTrigger::deserialize(trigger)?;
            if let Some(id) = self.mapping.get(&trigger) {
                if let Some(command) = self.commands.get_mut(id) {
                    command.use_dynamic_data(data)?;
                }
            }
        }
        Ok(())
    }

    pub fn add_payload<P>(&mut self, name: &str, payload: P) where P: PayloadFactory + 'static {
        self.payloads.insert(name.to_string(),Rc::new(Box::new(payload)));
    }

    pub(super) fn get_payloads(&self) -> &HashMap<String,Rc<Box<dyn PayloadFactory>>> { &self.payloads }

    pub(super) fn get_headers(&self) -> &HashMap<String,String> { &self.headers }

    fn cbor_trace(&self) -> CborValue {
        let mut opcodes = self.trace.keys().collect::<Vec<_>>();
        opcodes.sort();
        CborValue::Array(opcodes.iter().map(|c| {
            CborValue::Array(vec![
                CborValue::Integer(**c as i128),
                CborValue::Text(self.trace.get(*c).unwrap().to_string())
            ])
        }).collect())
    }

    pub(super) fn check_trace(&self) -> Result<(),String> {
        let got = checksum_iso(&serde_cbor::to_vec(&self.cbor_trace()).map_err(|_| format!("tracing failed"))?);
        if got != self.csi.trace() {
            Err(format!("trace comparison failed for {}: expected {:08X}, got {:08X}",self.id(),self.csi.trace(),got))
        } else {
            Ok(())
        }
    }

    pub fn push<T>(&mut self, name: &str, opcode: u32, commandtype: T) -> Result<(),String> where T: CommandType + 'static {
        if self.names.contains(name) {
            return Err(format!("Duplicate name {}",name));
        }
        self.names.insert(name.to_string());
        let schema = commandtype.get_schema();
        self.trace.insert(opcode,name.to_string());
        if self.commands.contains_key(&opcode) {
            return Err(format!("Duplicate opcode {}",opcode));
        }
        if self.mapping.contains_key(&schema.trigger) {
            return Err(format!("Duplicate trigger {}",schema.trigger));
        }
        self.commands.insert(opcode,Box::new(commandtype));
        self.mapping.insert(schema.trigger,opcode);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::commands::{
        ConstCommandType, NumberConstCommandType, BooleanConstCommandType, StringConstCommandType
    };
    use crate::generate::InstructionSuperType;

    #[test]
    fn test_command_smoke() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi,false);
        cs.push("test1",0,ConstCommandType::new()).expect("a");
        cs.push("test3",2,NumberConstCommandType::new()).expect("c");
        cs.push("test2",1,BooleanConstCommandType::new()).expect("b");
        cs.check_trace().expect("d");
        assert_eq!(&csi,cs.id());
    }

    #[test]
    fn test_command_get() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi,false);
        cs.push("test1",0,ConstCommandType::new()).expect("a");
        cs.push("test2",1,BooleanConstCommandType::new()).expect("b");
        cs.push("test3",2,NumberConstCommandType::new()).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cs.get(0).expect("d").get_schema().trigger);
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cs.get(2).expect("e").get_schema().trigger);
        assert!(cs.get(3).is_err());
    }

    fn trace_type(cs: &mut CommandSet, name: &str, opcode: u32) {
        match opcode {
            0 => cs.push(name,opcode,NumberConstCommandType::new()).expect("a"),
            1 => cs.push(name,opcode,BooleanConstCommandType::new()).expect("b"),
            2 => cs.push(name,opcode,ConstCommandType::new()).expect("c"),
            _ => cs.push(name,opcode,StringConstCommandType::new()).expect("d")
        }
    }

    fn trace_check(trace: u64, cmds: Vec<(&str,u32)>) -> bool {
        let csi = CommandSetId::new("test",(1,2),trace);
        let mut cs = CommandSet::new(&csi,false);
        for (name,opcode) in &cmds {
            trace_type(&mut cs, name, *opcode);
        }
        cs.check_trace().is_ok()
    }

    #[test]
    fn verify_trace() {
        assert!(trace_check(0x1E139093D228F8FF,vec![("test1",0),("test2",1),("test3",2)]));
        assert!(trace_check(0x1E139093D228F8FF,vec![("test2",1),("test1",0),("test3",2)]));
        assert!(!trace_check(0x1E139093D228F8FF,vec![("test1",0),("test3",2)]));
        assert!(!trace_check(0x1E139093D228F8FF,vec![("test2",1),("test1",0),("test3",2),("test4",3)]));
    }

    #[test]
    fn duplicate_opcode_test() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi,false);
        cs.push("test1",0,ConstCommandType::new()).expect("a");
        cs.push("test2",0,ConstCommandType::new()).expect_err("b");
    }

    #[test]
    fn duplicate_name_test() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi,false);
        cs.push("test1",0,ConstCommandType::new()).expect("a");
        cs.push("test1",1,ConstCommandType::new()).expect_err("b");
    }

    #[test]
    fn test_mappings() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi,false);
        cs.push("test1",0,ConstCommandType::new()).expect("a");
        cs.push("test2",1,NumberConstCommandType::new()).expect("c");
        let mappings = cs.get_mappings();
        assert_eq!(2,mappings.len());
        assert_eq!(Some(&0),mappings.get(&CommandTrigger::Instruction(InstructionSuperType::Const)));
        assert_eq!(Some(&1),mappings.get(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)));
    }
}
