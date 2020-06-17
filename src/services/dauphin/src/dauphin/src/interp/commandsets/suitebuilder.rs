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
use super::commandset::CommandSet;
use super::member::CommandSuiteMember;
use super::{ CommandCompileSuite, CommandInterpretSuite };
use crate::commands::{ make_core, make_library, make_buildtime };
use crate::interp::PayloadFactory;
use crate::cli::Config;
use serde_cbor::Value as CborValue;

pub struct LibrarySuiteBuilder {
    next_opcode: u32,
    headers: HashMap<String,String>,
    seen: HashMap<(String,u32),String>,
    compile_suite: CommandCompileSuite,
    interpret_suite: CommandInterpretSuite,
    payloads: HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>
}

impl LibrarySuiteBuilder {
    pub fn new() -> LibrarySuiteBuilder {
        LibrarySuiteBuilder {
            headers: HashMap::new(),
            compile_suite: CommandCompileSuite::new(),
            interpret_suite: CommandInterpretSuite::new(),
            seen: HashMap::new(),
            next_opcode: 0,
            payloads: HashMap::new()
        }
    }

    pub fn payloads(&self) -> &HashMap<(String,String),Rc<Box<dyn PayloadFactory>>> {
        &self.payloads
    }

    pub fn add(&mut self, mut set: CommandSet) -> Result<(),String> {
        let set_id = set.id().clone();
        let set_name = set_id.name().to_string();
        let set_major = set_id.version().0;
        if let Some(name) = self.seen.get(&(set_name.to_string(),set_major)) {
            return Err(format!("Attempt to register multiple versions {} and {}",set_id,name));
        }
        let mut mappings = set.get_mappings().clone();
        let set = Rc::new(set);
        let offset = self.next_opcode;
        self.compile_suite.add_set(set.clone(),offset);
        let set_offset = if set.compile_only() {
            None
        } else {
            Some(self.interpret_suite.add_set(set.clone()))
        };
        for (trigger,local_opcode) in mappings.drain() {
            let member = CommandSuiteMember::new(local_opcode,set.clone(),offset);
            if local_opcode+offset >= self.next_opcode {
                self.next_opcode = local_opcode+offset+1;
            }
            self.compile_suite.add_member(trigger,&member,set.compile_only());
            if let Some(set_offset) = set_offset {
                self.interpret_suite.add_member(offset+local_opcode,&member,set_offset);
            }
        }
        self.seen.insert((set_name.to_string(),set_major),set_id.to_string());
        for (name,value) in set.get_headers() {
            self.headers.insert(name.to_string(),value.to_string());
        }
        for (name,payload) in set.get_payloads() {
            self.payloads.insert((set_name.to_string(),name.to_string()),payload.clone());
        }
        Ok(())
    }

    pub fn get_headers(&self) -> &HashMap<String,String> { &self.headers }

    fn check_traces(&self) -> Result<(),String> {
        self.compile_suite.check_traces()
    }

    pub fn make_compile_suite(self) -> Result<CommandCompileSuite,String> { 
        self.check_traces()?;
        Ok(self.compile_suite)
    }

    pub fn make_interpret_suite(mut self, cbor: &CborValue) -> Result<CommandInterpretSuite,String> {
        self.check_traces()?;
        self.interpret_suite.adjust(cbor)?;
        Ok(self.interpret_suite)
    }
}

pub fn make_librarysuite_builder(config: &Config) -> Result<LibrarySuiteBuilder,String> {
    let mut suite = LibrarySuiteBuilder::new();
    suite.add(make_core()?)?;
    if !config.get_nostd() {
        suite.add(make_library()?)?;
    }
    if config.get_libs().contains(&"buildtime".to_string()) {
        suite.add(make_buildtime()?)?;
    }
    Ok(suite)
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CommandSetId, CommandTrigger };
    use crate::commands::{ ConstCommandType, NumberConstCommandType };
    use crate::generate::InstructionSuperType;
    use crate::interp::{ xxx_test_config, make_librarysuite_builder, serialize };

    #[test]
    fn test_suite_smoke() {
        /* imagine all this at the compiler end */
        let mut cb = LibrarySuiteBuilder::new();

        //
        let csi1 = CommandSetId::new("test",(1,2),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        cb.add(cs1).expect("f");
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2,false);
        cs2.push("test2",5,NumberConstCommandType()).expect("a");
        cb.add(cs2).expect("f");
        //
        let ccs = cb.make_compile_suite().expect("f");
        let (_,opcode,_) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::Const)).expect("c");
        assert_eq!(5,opcode);
        let (_,opcode,_) = ccs.get_by_trigger(&CommandTrigger::Instruction(InstructionSuperType::NumberConst)).expect("c");
        assert_eq!(11,opcode);

        /* and here's the same thing, but with sets flipped, at the interpreter end */
        let mut cb = LibrarySuiteBuilder::new();
        //
        let csi2 = CommandSetId::new("test2",(1,2),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2,false);
        cs2.push("test2",5,NumberConstCommandType()).expect("a");
        cb.add(cs2).expect("f");
        //
        let csi1 = CommandSetId::new("test",(1,2),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        cb.add(cs1).expect("f");
        //
        let cis = cb.make_interpret_suite(&ccs.serialize()).expect("g");
        
        /* now, our opcodes should be flipped to match ccs */
        let cmd = cis.get_by_opcode(5).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
        let cmd = cis.get_by_opcode(11).expect("d");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::NumberConst),cmd.get_schema().trigger);
    }

    fn age_check(compiler: (u32,u32), interpreter: (u32,u32)) -> bool {
        let mut cb = LibrarySuiteBuilder::new();

        let csi1 = CommandSetId::new("test",compiler,0xB790000000000000);
        let cs1 = CommandSet::new(&csi1,false);
        cb.add(cs1).expect("a");
        let ccs = cb.make_compile_suite().expect("b");

        let mut cb = LibrarySuiteBuilder::new();
        let csi1 = CommandSetId::new("test",interpreter,0xB790000000000000);
        let cs1 = CommandSet::new(&csi1,false);
        cb.add(cs1).expect("c");
        cb.make_interpret_suite(&ccs.serialize()).is_ok()
    }

    #[test]
    fn test_interp_too_old() {
        assert!(age_check((1,1),(1,1)));
        assert!(age_check((1,1),(1,2))); /* compiler can be behing interpreter in a minor number */
        assert!(!age_check((1,2),(1,1))); /* but not the other way round */
        assert!(!age_check((1,1),(2,1))); /* and not by a major number */
    }

    #[test]
    fn test_no_multi_minor() {
        let mut cb = LibrarySuiteBuilder::new();

        let csi1 = CommandSetId::new("test",(1,1),0xB790000000000000);
        let cs1 = CommandSet::new(&csi1,false);
        cb.add(cs1).expect("a");
        let csi1 = CommandSetId::new("test",(1,2),0xB790000000000000);
        let cs1 = CommandSet::new(&csi1,false);
        cb.add(cs1).expect_err("a");
    }

    #[test]
    fn test_ok_multi_major() {
        let mut cb = LibrarySuiteBuilder::new();

        let csi1 = CommandSetId::new("test",(1,1),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        cb.add(cs1).expect("a");

        let ccs = cb.make_compile_suite().expect("b");

        let mut cb = LibrarySuiteBuilder::new();
        let csi2 = CommandSetId::new("test",(2,1),0x284E7C72C8628854);
        let mut cs2 = CommandSet::new(&csi2,false);
        cs2.push("test2",5,NumberConstCommandType()).expect("a");
        cb.add(cs2).expect("c");
        let csi1 = CommandSetId::new("test",(1,1),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        cb.add(cs1).expect("c");

        let cis = cb.make_interpret_suite(&ccs.serialize()).expect("c");
        let cmd = cis.get_by_opcode(5).expect("c");
        assert_eq!(CommandTrigger::Instruction(InstructionSuperType::Const),cmd.get_schema().trigger);
    }

    #[test]
    fn test_missing_set_bad_interp() {
        let mut cb = LibrarySuiteBuilder::new();

        let csi1 = CommandSetId::new("test",(1,1),0x2A9E7C72C8628854);
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        cb.add(cs1).expect("a");

        let ccs = cb.make_compile_suite().expect("b");

        let cb = LibrarySuiteBuilder::new();
        assert!(cb.make_interpret_suite(&ccs.serialize()).is_err());
    }

    #[test]
    fn test_missing_set_ok_compiler() {
        let cb = LibrarySuiteBuilder::new();

        let csi1 = CommandSetId::new("test",(1,1),0x2A9E7C72C8628854);
        let ccs = cb.make_compile_suite().expect("b");

        let mut cb = LibrarySuiteBuilder::new();
        let mut cs1 = CommandSet::new(&csi1,false);
        cs1.push("test1",5,ConstCommandType()).expect("a");
        cb.add(cs1).expect("a");

        assert!(cb.make_interpret_suite(&ccs.serialize()).is_ok());
    }

    #[test]
    fn test_dynamic_data() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_verbose(2);
        let builder = make_librarysuite_builder(&config).expect("y");
        let data = builder.make_compile_suite().expect("z").generate_dynamic_data(&config).expect("x");
        for (suite,data) in data.iter() {
            print!("command set {}\n",suite);
            serialize(&data).expect("a");
        }
    }
}
