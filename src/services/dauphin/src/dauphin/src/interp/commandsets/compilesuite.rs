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
use std::collections::{ HashMap, BTreeMap };
use super::command::{ CommandTrigger, CommandType };
use dauphin_interp_common::common::{ CommandSetId, CommandDeserializer, cbor_map_iter };
use crate::interp::{ CompilerLink, CommandTypeStore, CommandTypeId };
use serde_cbor::Value as CborValue;
use super::suite::{ CommandSetVerifier, CompLibRegister, OpcodeMapping };

pub struct CommandCompileSuite {
    store: CommandTypeStore,
    sets: Vec<CommandSetId>,
    set_commands: HashMap<CommandSetId,Vec<CommandTypeId>>,
    command_offsets: HashMap<CommandTypeId,(CommandSetId,u32)>,
    command_triggers: HashMap<CommandTypeId,CommandTrigger>,
    trigger_commands: HashMap<CommandTrigger,CommandTypeId>,
    interp_commands: HashMap<CommandTypeId,Box<dyn CommandDeserializer>>,
    opcode_mapper: OpcodeMapping,
    headers: HashMap<String,String>,
    verifier: CommandSetVerifier
}

impl CommandCompileSuite {
    pub(super) fn new() -> CommandCompileSuite {
        CommandCompileSuite {
            store: CommandTypeStore::new(),
            sets: vec![],
            set_commands: HashMap::new(),
            command_triggers: HashMap::new(),
            command_offsets: HashMap::new(),
            trigger_commands: HashMap::new(),
            interp_commands: HashMap::new(),
            headers: HashMap::new(),
            opcode_mapper: OpcodeMapping::new(),
            verifier: CommandSetVerifier::new()        
        }
    }

    pub fn register(&mut self, mut set: CompLibRegister) -> Result<(),String> {
        set.check_trace()?;
        let sid = set.id().clone();
        self.sets.push(sid.clone());
        let mut offset_interp_command = HashMap::new();
        if let Some(mut ils) = set.drain_interp_lib_register() {
            for ds in ils.drain_commands() {
                if let Some(offset) = ds.get_opcode_len()? {
                    offset_interp_command.insert(offset.0,ds);
                }
            }
        } else {
            self.opcode_mapper.dont_serialize(&sid);
        }
        for (offset,command) in set.drain_commands().drain(..) {
            let cid = self.store.add(command).clone();
            if let Some(offset) = offset {
                if let Some(interp_command) = offset_interp_command.remove(&offset) {
                    self.interp_commands.insert(cid.clone(),interp_command);
                }
                self.opcode_mapper.add_opcode(&sid,offset);
                self.command_offsets.insert(cid.clone(),(sid.clone(),offset));
            }
            self.set_commands.entry(sid.clone()).or_insert_with(|| vec![]).push(cid.clone());
            let schema = self.store.get(&cid).get_schema();
            self.command_triggers.insert(cid.clone(),schema.trigger.clone());
            self.trigger_commands.insert(schema.trigger.clone(),cid.clone());
        }
        self.opcode_mapper.recalculate();
        for (name,value) in set.drain_headers().drain(..) {
            self.headers.insert(name.to_string(),value.to_string());
        }
        for data in set.drain_dynamic_data().iter() {
            self.load_dynamic_data(&sid,&data)?;
        }
        self.verifier.register2(&sid)?;
        Ok(())
    }

    pub fn get_headers(&self) -> &HashMap<String,String> { &self.headers }

    pub fn get_set_ids(&self) -> Vec<CommandSetId> {
        self.sets.iter().cloned().collect()
    }

    pub fn serialize(&self) -> CborValue {
        self.opcode_mapper.serialize()
    }

    pub fn get_command_by_trigger(&self, trigger: &CommandTrigger) -> Result<&Box<dyn CommandType>,String> {
        let cid = self.trigger_commands.get(trigger).ok_or(format!("Unknown command {}",trigger))?;
        let cmdtype = self.store.get(cid);
        Ok(cmdtype)
    }

    pub fn get_deserializer_by_trigger(&self, trigger: &CommandTrigger) -> Result<Option<&Box<dyn CommandDeserializer>>,String> {
        let cid = self.trigger_commands.get(trigger).ok_or(format!("Unknown command {:?}",trigger))?;
        Ok(self.interp_commands.get(cid))
    }

    pub fn get_opcode_by_trigger(&self, trigger: &CommandTrigger) -> Result<Option<u32>,String> {
        let cid = self.trigger_commands.get(trigger).ok_or(format!("Unknown command {}",trigger))?;
        if let Some((sid,offset)) = self.command_offsets.get(cid) {
            Ok(Some(self.opcode_mapper.sid_to_offset(sid)?+offset))
        } else {
            Ok(None)
        }
    }

    pub fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<HashMap<CommandSetId,CborValue>,String> {
        let mut out = HashMap::new();
        for (set,commands) in self.set_commands.iter() {
            if config.get_verbose() > 0 {
                print!("generating dynamic data for {}/{}.{}\n",set.name(),set.version().0,set.version().1);
            }
            let mut set_data = BTreeMap::new();
            for cid in commands.iter() {
                if let Some(trigger) = self.command_triggers.get(cid) {
                    if config.get_verbose() > 1 {
                        print!("dynamic data for {}\n",trigger);
                    }        
                    let command = self.store.get(cid);
                    set_data.insert(trigger.serialize(),command.generate_dynamic_data(linker,config)?);
                }
            }
            out.insert(set.clone(),CborValue::Map(set_data));
        }
        Ok(out)
    }

    pub fn load_dynamic_data(&mut self, set: &CommandSetId, data: &[u8]) -> Result<(),String> {
        if let Some(commands) = self.set_commands.get(set) {
            let data : CborValue = serde_cbor::from_slice(&data).map_err(|x| format!("{} while deserialising {:?}",x,set))?;
            for (trigger,data) in cbor_map_iter(&data)? {
                let trigger = CommandTrigger::deserialize(trigger)?;
                if let Some(cid) = self.trigger_commands.get(&trigger) {
                    let command = self.store.get_mut(&cid);
                    command.use_dynamic_data(data).unwrap_or_else(|_| {
                        eprint!("Cannot load dynamic data for {:?}\n",trigger);
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CommandTrigger };
    use dauphin_interp_common::common::{ CommandSetId, NoopDeserializer };
    use dauphin_interp_common::interp::{ InterpLibRegister };
    use crate::commands::{ ConstCommandType, NumberConstCommandType };
    use crate::generate::InstructionSuperType;
    use crate::test::cbor::cbor_cmp;

    #[test]
    fn test_compile_smoke() {
        let mut ccs = CommandCompileSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x1C5F9E7C72C86288);
        let mut is1 = InterpLibRegister::new(&csi1);
        is1.push(NoopDeserializer(5));
        let mut cs1 = CompLibRegister::new(&csi1,Some(is1));
        cs1.push("test1",Some(5),ConstCommandType::new());
        ccs.register(cs1).expect("a1");
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x1C5D4E7C72C86288);
        let mut is2 = InterpLibRegister::new(&csi1);
        is2.push(NoopDeserializer(5));
        let mut cs2 = CompLibRegister::new(&csi2,Some(is2));
        cs2.push("test2",Some(5),NumberConstCommandType::new());
        ccs.register(cs2).expect("a2");
        //
        let cmd = ccs.get_command_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::Const)).expect("b");
        let opcode = ccs.get_opcode_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::Const)).expect("b");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
        assert_eq!(Some(5),opcode);
        let cmd = ccs.get_command_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("c");
        let opcode = ccs.get_opcode_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cmd.get_schema().trigger);
        assert_eq!(Some(11),opcode);
        cbor_cmp(&ccs.serialize(),"compilesuite.out");
    }
}