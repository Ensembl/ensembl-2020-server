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

use std::rc::Rc;
use crate::cli::Config;
use std::collections::{ HashMap, BTreeMap };
use crate::command::{ CommandTrigger, CommandType };
use dauphin_interp::command::{ CommandSetId, CommandDeserializer, CommandTypeId, OpcodeMapping, CommandSetVerifier };
use dauphin_interp::runtime::{ PayloadFactory };
use dauphin_interp::util::cbor::{ cbor_map_iter };
use crate::command::{ CompilerLink, CommandTypeStore, CompLibRegister };
use serde_cbor::Value as CborValue;

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
    verifier: CommandSetVerifier,
    payloads: HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>
}

impl CommandCompileSuite {
    pub fn new() -> CommandCompileSuite {
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
            verifier: CommandSetVerifier::new(),
            payloads: HashMap::new()
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
            self.payloads.extend(&mut ils.drain_payloads().drain());
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

    pub fn copy_payloads(&self) -> HashMap<(String,String),Rc<Box<dyn PayloadFactory>>> {
        self.payloads.clone()
    }

    pub fn get_set_ids(&self) -> Vec<CommandSetId> {
        self.sets.iter().cloned().collect()
    }

    pub fn serialize(&self) -> CborValue {
        self.opcode_mapper.serialize()
    }

    pub fn get_command_by_trigger(&self, trigger: &CommandTrigger) -> Result<&Box<dyn CommandType>,String> {
        let cid = self.trigger_commands.get(trigger).ok_or(format!("Unknown command/1 {}",trigger))?;
        let cmdtype = self.store.get(cid);
        Ok(cmdtype)
    }

    pub fn get_deserializer_by_trigger(&self, trigger: &CommandTrigger) -> Result<Option<&Box<dyn CommandDeserializer>>,String> {
        let cid = self.trigger_commands.get(trigger).ok_or(format!("Unknown command/2 {:?}",trigger))?;
        Ok(self.interp_commands.get(cid))
    }

    pub fn get_opcode_by_trigger(&self, trigger: &CommandTrigger) -> Result<Option<u32>,String> {
        let cid = self.trigger_commands.get(trigger).ok_or(format!("Unknown command/3 {}",trigger))?;
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
        if self.set_commands.contains_key(set) {
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
    use std::cell::RefCell;
    use crate::command::{ CommandTrigger, Command, Instruction, InstructionSuperType };
    use dauphin_interp::command::{ CommandSetId, InterpLibRegister, Identifier, CommandInterpretSuite };
    use dauphin_interp::runtime::{ InterpContext };
    use dauphin_interp::util::templates::NoopDeserializer;
    use crate::command::{ CommandSchema, CommandType };
    use crate::test::{ cbor_cmp, FakeDeserializer };
    use crate::core::{ NumberConstCommandType, ConstCommandType, BooleanConstCommandType, StringConstCommandType };

    fn fake_trigger(name: &str) -> CommandTrigger {
        CommandTrigger::Command(Identifier::new("fake",name))
    }

    struct FakeCommandType(String);

    impl CommandType for FakeCommandType {
        fn get_schema(&self) -> CommandSchema {
            CommandSchema {
                trigger: fake_trigger(&self.0),
                values: 0
            }
        }

        fn from_instruction(&self, _it: &Instruction) -> Result<Box<dyn Command>,String> {
            Err(format!("unable"))
        }
    }

    fn fake_command(name: &str) -> impl CommandType {
        FakeCommandType(name.to_string())
    }

    #[test]
    fn test_compile_smoke() {
        let mut ccs = CommandCompileSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x1F3F9E7C72C86288);
        let mut is1 = InterpLibRegister::new(&csi1);
        is1.push(NoopDeserializer(5));
        let mut cs1 = CompLibRegister::new(&csi1,Some(is1));
        cs1.push("test1",Some(5),fake_command("c"));
        ccs.register(cs1).expect("a1");
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x1F3D4E7C72C86288);
        let mut is2 = InterpLibRegister::new(&csi1);
        is2.push(NoopDeserializer(5));
        let mut cs2 = CompLibRegister::new(&csi2,Some(is2));
        cs2.push("test2",Some(5),fake_command("nc"));
        ccs.register(cs2).expect("a2");
        //
        let cmd = ccs.get_command_by_trigger(&fake_trigger("c")).expect("b");
        let opcode = ccs.get_opcode_by_trigger(&fake_trigger("c")).expect("b");
        assert_eq!(fake_trigger("c"),cmd.get_schema().trigger);
        assert_eq!(Some(5),opcode);
        let cmd = ccs.get_command_by_trigger(&fake_trigger("nc")).expect("c");
        let opcode = ccs.get_opcode_by_trigger(&fake_trigger("nc")).expect("c");
        assert_eq!(fake_trigger("nc"),cmd.get_schema().trigger);
        assert_eq!(Some(11),opcode);
        cbor_cmp(&ccs.serialize(),"compilesuite.out");
    }

    #[test]
    fn test_interpretsuite_smoke() {
        let v : Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        /* imagine all this at the compiler end */
        let mut ccs = CommandCompileSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x1F3F9E7C72C86288);
        let mut is1 = InterpLibRegister::new(&csi1);
        is1.push(FakeDeserializer(v.clone(),5));
        let mut cs1 = CompLibRegister::new(&csi1,Some(is1));
        cs1.push("test1",Some(5),fake_command("c"));
        ccs.register(cs1).expect("a");
        //
        let csi2 = CommandSetId::new("test2",(1,2),0xB03D4E7C72C8628A);
        let mut is2 = InterpLibRegister::new(&csi2);
        is2.push(FakeDeserializer(v.clone(),6));
        let mut cs2 = CompLibRegister::new(&csi2,Some(is2));
        cs2.push("test2",Some(6),fake_command("nc"));
        ccs.register(cs2).expect("b");
        let opcode = ccs.get_opcode_by_trigger(&fake_trigger("nc")).expect("c");
        assert_eq!(Some(12),opcode);

        /* and here's the same thing, but subtly rearranged, at the interpreter end */
        let mut cis = CommandInterpretSuite::new();
        //
        let csi1 = CommandSetId::new("test",(1,2),0x1F3F9E7C72C86288);
        let mut cs1 = InterpLibRegister::new(&csi1);
        cs1.push(FakeDeserializer(v.clone(),5));
        cis.register(cs1).expect("c");
        //
        let csi3 = CommandSetId::new("test3",(1,2),0x5B5E7C72C8628B34);
        let mut cs3 = InterpLibRegister::new(&csi3);
        cs3.push(FakeDeserializer(v.clone(),7));
        cis.register(cs3).expect("c");
        //
        let csi2 = CommandSetId::new("test2",(1,2),0xB03D4E7C72C8628A);
        let mut cs2 = InterpLibRegister::new(&csi2);
        cs2.push(FakeDeserializer(v.clone(),6));
        cis.register(cs2).expect("c");

        cis.adjust(&ccs.serialize()).expect("e");
        //print!("{:?}\n",cis.offset_to_command);

        let mut context = InterpContext::new(&HashMap::new());
        cis.get_deserializer(5).expect("e").deserialize(5,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(5,*v.borrow());
        cis.get_deserializer(12).expect("e").deserialize(12,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(6,*v.borrow());
    }

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
