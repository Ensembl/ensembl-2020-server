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
use std::collections::{ HashMap, HashSet, BTreeMap };
use std::rc::Rc;
use super::command::{ CommandTrigger, CommandType };
use super::commandset::CommandSet;
use super::commandsetid::CommandSetId;
use super::member::CommandSuiteMember;
use crate::interp::CompilerLink;
use serde_cbor::Value as CborValue;

pub struct CommandCompileSuite {
    sets: Vec<(Rc<CommandSet>,u32)>,
    mapping: HashMap<CommandTrigger,CommandSuiteMember>,
    compile_only: HashSet<CommandTrigger>
}

impl CommandCompileSuite {
    pub(super) fn new() -> CommandCompileSuite {
        CommandCompileSuite {
            sets: vec![],
            mapping: HashMap::new(),
            compile_only: HashSet::new()
        }
    }

    pub(super) fn add_set(&mut self, set: Rc<CommandSet>, offset: u32) {
        self.sets.push((set,offset));
    }

    pub fn get_set_ids(&self) -> Vec<CommandSetId> {
        self.sets.iter().map(|x| x.0.id().clone()).collect()
    }

    pub(super) fn add_member(&mut self, trigger: CommandTrigger, member: &CommandSuiteMember, compile_only: bool) {
        self.mapping.insert(trigger.clone(),member.clone());
        if compile_only {
            self.compile_only.insert(trigger);
        }
    }

    pub fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<HashMap<CommandSetId,CborValue>,String> {
        let mut out = HashMap::new();
        for (set,_) in self.sets.iter() {
            if config.get_verbose() > 0 {
                print!("generating dynamic data for {}/{}.{}\n",set.id().name(),set.id().version().0,set.id().version().1);
            }
            out.insert(set.id().clone(),set.generate_dynamic_data(linker,config)?);
        }
        Ok(out)
    }

    pub(super) fn check_traces(&self) -> Result<(),String> {
        for members in &self.sets {
            members.0.check_trace()?;
        }
        Ok(())
    }

    pub fn serialize(&self) -> CborValue {
        let mut out = vec![];
        for members in &self.sets {
            if !members.0.compile_only() {
                out.push(CborValue::Integer(members.1 as i128));
                out.push(members.0.id().serialize());
            }
        }
        CborValue::Array(out)
    }

    pub fn get_by_trigger(&self, trigger: &CommandTrigger) -> Result<(&Box<dyn CommandType>,u32,bool),String> {
        let member = self.mapping.get(trigger).ok_or(format!("Unknown command {}",trigger))?;
        let cmdtype = member.get_object()?;
        Ok((cmdtype,member.opcode(),self.compile_only.contains(trigger)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CommandSetId, CommandTrigger };
    use crate::commands::{ ConstCommandType, NumberConstCommandType };
    use crate::generate::InstructionSuperType;
    use crate::test::cbor::cbor_cmp;

    #[test]
    fn test_compile_smoke() {
        let mut ccs = CommandCompileSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType::new()).expect("a");
        let cs1 = Rc::new(cs1);
        ccs.add_set(cs1.clone(),10);
        let m = CommandSuiteMember::new(5,cs1.clone(),10);
        ccs.add_member(CommandTrigger::Instruction(InstructionSuperType::Const),&m,false);
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2,false);
        cs2.push("test2",5,NumberConstCommandType::new()).expect("a");
        let cs2 = Rc::new(cs2);
        ccs.add_set(cs2.clone(),20);
        let m = CommandSuiteMember::new(5,cs2.clone(),20);
        ccs.add_member(CommandTrigger::Instruction(InstructionSuperType::NumberConst),&m,false);
        //
        let (cmd,opcode,_) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::Const)).expect("b");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
        assert_eq!(15,opcode);
        let (cmd,opcode,_) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cmd.get_schema().trigger);
        assert_eq!(25,opcode);
        ccs.check_traces().expect("c");
        cbor_cmp(&ccs.serialize(),"compilesuite.out");
    }
}