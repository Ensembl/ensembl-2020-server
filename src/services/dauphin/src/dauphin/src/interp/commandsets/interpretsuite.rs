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
 *  
 *  vscode-fold=1
 */

use std::collections::HashMap;
use std::rc::Rc;
use super::command::CommandType;
use super::commandset::CommandSet;
use super::member::CommandSuiteMember;
use super::CommandSetId;
use crate::model::{ cbor_array, cbor_int };
use serde_cbor::Value as CborValue;

pub struct CommandInterpretSuite {
    sets: Vec<Rc<CommandSet>>,
    opcodes: HashMap<u32,(CommandSuiteMember,usize)>
}

impl CommandInterpretSuite {
    pub(super) fn new() -> CommandInterpretSuite {
        CommandInterpretSuite {
            sets: vec![],
            opcodes: HashMap::new()
        }
    }

    fn deserialize(&self, cbor: &CborValue) -> Result<HashMap<CommandSetId,u32>,String> {
        let data = cbor_array(cbor,0,true)?;
        if data.len()%2 != 0 {
            return Err(format!("badly formed cbor"))
        }
        let mut out = HashMap::new();
        for i in (0..data.len()).step_by(2) {
            let offset = 
            out.insert(CommandSetId::deserialize(&data[i+1])?,cbor_int(&data[i],None)? as u32);
        }
        Ok(out)
    }

    pub(super) fn add_member(&mut self, real_opcode: u32, member: &CommandSuiteMember, set_index: usize) {
        self.opcodes.insert(real_opcode,(member.clone(),set_index));
    }

    pub(super) fn add_set(&mut self, set: Rc<CommandSet>) -> usize {
        let out = self.sets.len();
        self.sets.push(set);
        out
    }

    pub(super) fn adjust(&mut self, cbor: &CborValue) -> Result<(),String> {
        let stored_id_list = self.sets.iter().map(|s| s.id()).enumerate()
            .map(|(idx,id)| {
                let version = id.version();
                ((id.name(),version.0),(idx,version.1))
            })
            .collect::<HashMap<_,_>>();
        let mut offsets = HashMap::new();
        for (incoming_id,offset) in self.deserialize(cbor)? {
            let name = incoming_id.name();
            let version = incoming_id.version();
            if let Some((set_index,minor)) = stored_id_list.get(&(name,version.0)) {
                if *minor < version.1 {
                    return Err(format!("version too old. have {} need {}",self.sets[*set_index].id(),incoming_id))
                }
                offsets.insert(*set_index,offset);
            } else {
                return Err(format!("missing command suite {}",incoming_id));
            }
        }
        let mut new_opcodes = HashMap::new();
        for (_,(mut member,index)) in self.opcodes.drain() {
            if let Some(offset) = offsets.get(&index) {
                member.set_offset(*offset);
                new_opcodes.insert(member.opcode(),(member,index));
            }
        }
        self.opcodes = new_opcodes;
        Ok(())
    }

    pub fn get_by_opcode(&self, real_opcode: u32) -> Result<&Box<dyn CommandType>,String> {
        let member = self.opcodes.get(&real_opcode).ok_or(format!("Unknown opcode {}",real_opcode))?;
        member.0.get_object()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CommandSetId, CommandTrigger, CommandCompileSuite };
    use crate::interp::commands::core::consts::{ ConstCommandType, NumberConstCommandType };
    use crate::generate::InstructionSuperType;


    #[test]
    fn test_interpretsuite_smoke() {
        /* imagine all this at the compiler end */
        let mut ccs = CommandCompileSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        let cs1 = Rc::new(cs1);
        ccs.add_set(cs1.clone(),10);
        let m = CommandSuiteMember::new(5,cs1.clone(),10);
        ccs.add_member(CommandTrigger::Instruction(InstructionSuperType::Const),&m);
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2);
        cs2.push("test2",5,NumberConstCommandType()).expect("a");
        let cs2 = Rc::new(cs2);
        ccs.add_set(cs2.clone(),20);
        let m = CommandSuiteMember::new(5,cs2.clone(),20);
        ccs.add_member(CommandTrigger::Instruction(InstructionSuperType::NumberConst),&m);
        let (_,opcode) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("c");
        assert_eq!(25,opcode);

        /* and here's the same thing, but subtly rearranged, at the interpreter end */
        let mut cis = CommandInterpretSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        let cs1 = Rc::new(cs1);
        let cs1i = cis.add_set(cs1.clone());
        let m = CommandSuiteMember::new(5,cs1.clone(),10);
        cis.add_member(15,&m,cs1i);
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2);
        cs2.push("test2",5,NumberConstCommandType()).expect("a");
        let cs2 = Rc::new(cs2);
        let cs2i = cis.add_set(cs2.clone());
        let m = CommandSuiteMember::new(5,cs2.clone(),30);
        cis.add_member(35,&m,cs2i);

        /* now, our opcodes should be 15/35, ie local assignments, for now */
        let cmd = cis.get_by_opcode(15).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
        let cmd = cis.get_by_opcode(35).expect("d");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cmd.get_schema().trigger);
        assert!(cis.get_by_opcode(25).is_err());

        cis.adjust(&ccs.serialize()).expect("e");
        /* but after magic adjustment, should be 15/25 */
        let cmd = cis.get_by_opcode(15).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
        let cmd = cis.get_by_opcode(25).expect("d");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cmd.get_schema().trigger);
        assert!(cis.get_by_opcode(35).is_err());
    }
}
