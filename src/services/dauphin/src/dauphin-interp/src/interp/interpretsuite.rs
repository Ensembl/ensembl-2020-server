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
use dauphin_interp_common::interp::{ CommandTypeId };
use crate::interp::Deserializer;
use dauphin_interp_common::common::{ cbor_array, cbor_int, CommandDeserializer, CommandSetId };
use dauphin_interp_common::interp::{ InterpLibRegister, OpcodeMapping, CommandSetVerifier, PayloadFactory };

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

#[cfg(test)]
mod test {
    use std::cell::RefCell;
    use std::rc::Rc;
    use super::*;
    use dauphin_interp_common::common::{ CommandSetId };
    use dauphin_interp_common::interp::{ InterpContext, InterpLibRegister };
    use dauphin_compile_common::command::{ CommandTrigger };
    use dauphin_compile_common::model::{ InstructionSuperType, CommandCompileSuite, CompLibRegister };
    use crate::test::{ FakeDeserializer, fake_command, fake_trigger };

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
        print!("{:?}\n",cis.offset_to_command);

        let mut context = InterpContext::new(&HashMap::new());
        cis.get_deserializer(5).expect("e").deserialize(5,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(5,*v.borrow());
        cis.get_deserializer(12).expect("e").deserialize(12,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(6,*v.borrow());
    }
}
