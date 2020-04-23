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

use std::collections::{ HashMap, HashSet }; // hashbrown
use super::CommandSetId;
use super::command::{ CommandType, CommandTrigger };
use serde_cbor::Value as CborValue;
use crc::crc64::checksum_iso;

pub struct CommandSet {
    names: HashSet<String>,
    trace: HashMap<u32,String>,
    csi: CommandSetId,
    commands: HashMap<u32,Box<dyn CommandType>>,
    mapping: Option<HashMap<CommandTrigger,u32>>
}

impl CommandSet {
    pub fn new(csi: &CommandSetId) -> CommandSet {
        CommandSet {
            names: HashSet::new(),
            trace: HashMap::new(),
            csi: csi.clone(),
            commands: HashMap::new(),
            mapping: Some(HashMap::new())
        }
    }

    pub(super) fn id(&self) -> &CommandSetId { &self.csi }
    pub(super) fn take_mappings(&mut self) -> HashMap<CommandTrigger,u32> {
        self.mapping.take().unwrap()
    }
    pub(super) fn get(&self, opcode: u32) -> Result<&Box<dyn CommandType>,String> {
        self.commands.get(&opcode).ok_or_else(|| format!("No such opcode {}",opcode))
    }

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
        if self.mapping.as_ref().unwrap().contains_key(&schema.trigger) {
            return Err(format!("Duplicate trigger {}",schema.trigger));
        }
        self.commands.insert(opcode,Box::new(commandtype));
        self.mapping.as_mut().unwrap().insert(schema.trigger,opcode);
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
        let mut cs = CommandSet::new(&csi);
        cs.push("test1",0,ConstCommandType()).expect("a");
        cs.push("test3",2,NumberConstCommandType()).expect("c");
        cs.push("test2",1,BooleanConstCommandType()).expect("b");
        cs.check_trace().expect("d");
        assert_eq!(&csi,cs.id());
    }

    #[test]
    fn test_command_get() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi);
        cs.push("test1",0,ConstCommandType()).expect("a");
        cs.push("test2",1,BooleanConstCommandType()).expect("b");
        cs.push("test3",2,NumberConstCommandType()).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cs.get(0).expect("d").get_schema().trigger);
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cs.get(2).expect("e").get_schema().trigger);
        assert!(cs.get(3).is_err());
    }

    fn trace_type(cs: &mut CommandSet, name: &str, opcode: u32) {
        match opcode {
            0 => cs.push(name,opcode,NumberConstCommandType()).expect("a"),
            1 => cs.push(name,opcode,BooleanConstCommandType()).expect("b"),
            2 => cs.push(name,opcode,ConstCommandType()).expect("c"),
            _ => cs.push(name,opcode,StringConstCommandType()).expect("d")
        }
    }

    fn trace_check(trace: u64, cmds: Vec<(&str,u32)>) -> bool {
        let csi = CommandSetId::new("test",(1,2),trace);
        let mut cs = CommandSet::new(&csi);
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
        let mut cs = CommandSet::new(&csi);
        cs.push("test1",0,ConstCommandType()).expect("a");
        cs.push("test2",0,ConstCommandType()).expect_err("b");
    }

    #[test]
    fn duplicate_name_test() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi);
        cs.push("test1",0,ConstCommandType()).expect("a");
        cs.push("test1",1,ConstCommandType()).expect_err("b");
    }

    #[test]
    fn test_mappings() {
        let csi = CommandSetId::new("test",(1,2),0x1E139093D228F8FF);
        let mut cs = CommandSet::new(&csi);
        cs.push("test1",0,ConstCommandType()).expect("a");
        cs.push("test2",1,NumberConstCommandType()).expect("c");
        let mappings = cs.take_mappings();
        assert_eq!(2,mappings.len());
        assert_eq!(Some(&0),mappings.get(&CommandTrigger::Instruction(InstructionSuperType::Const)));
        assert_eq!(Some(&1),mappings.get(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)));
    }
}
