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

use dauphin_compile::cli::Config;
use dauphin_compile::model::{ CommandCompileSuite };
use dauphin_interp::interp::{ CommandInterpretSuite };
use dauphin_lib_std::make_std_interp;
use dauphin_lib_std::make_std;
use dauphin_interp::make_core_interp;
use dauphin_compile::commands::{ make_core };
use dauphin_lib_buildtime::{ make_buildtime };

pub fn make_compiler_suite(config: &Config) -> Result<CommandCompileSuite,String> {
    let mut suite = CommandCompileSuite::new();
    suite.register(make_core()?)?;
    if !config.get_nostd() {
        suite.register(make_std()?)?;
    }
    if config.get_libs().contains(&"buildtime".to_string()) {
        suite.register(make_buildtime()?)?;
    }
    Ok(suite)
}

pub fn make_interpret_suite(config: &Config) -> Result<CommandInterpretSuite,String> {
    let mut suite = CommandInterpretSuite::new();
    suite.register(make_core_interp()?)?;
    if !config.get_nostd() {
        suite.register(make_std_interp()?)?;
    }
    Ok(suite)
}

#[cfg(test)]
mod test {
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use super::*;
    use dauphin_interp::common::{ CommandSetId, NoopDeserializer, cbor_serialize };
    use dauphin_interp::interp::{ InterpLibRegister, InterpContext };
    use dauphin_compile::model::{ CompilerLink, CompLibRegister };
    use crate::test::{ FakeDeserializer, fake_command, xxx_test_config, fake_trigger };

    // XXX dedup
    
    #[test]
    fn test_suite_smoke() {
        let v : Rc<RefCell<u32>> = Rc::new(RefCell::new(0));

        /* imagine all this at the compiler end */
        let mut ccs = CommandCompileSuite::new();

        //
        let csi1 = CommandSetId::new("test",(1,2),0x1F3F9E7C72C86288);
        let mut is1 = InterpLibRegister::new(&csi1);
        is1.push(FakeDeserializer(v.clone(),5));
        let mut cs1 = CompLibRegister::new(&csi1,Some(is1));
        cs1.push("test1",Some(5),fake_command("c"));
        ccs.register(cs1).expect("f");
        //
        let csi2 = CommandSetId::new("test2",(1,2),0xB03D4E7C72C8628A);
        let mut is2 = InterpLibRegister::new(&csi2);
        is2.push(FakeDeserializer(v.clone(),6));
        let mut cs2 = CompLibRegister::new(&csi2,Some(is2));
        cs2.push("test2",Some(6),fake_command("nc"));
        ccs.register(cs2).expect("f");
        //
        let opcode = ccs.get_opcode_by_trigger(&fake_trigger("c")).expect("c");
        assert_eq!(Some(5),opcode);
        let opcode = ccs.get_opcode_by_trigger(&fake_trigger("nc")).expect("c");
        assert_eq!(Some(12),opcode);

        
        /* and here's the same thing, but with sets flipped, at the interpreter end */
        let mut cis = CommandInterpretSuite::new();
        //
        let csi2 = CommandSetId::new("test2",(1,2),0xB03D4E7C72C8628A);
        let mut cs2 = InterpLibRegister::new(&csi2);
        cs2.push(FakeDeserializer(v.clone(),6));
        cis.register(cs2).expect("f");
        //
        let csi1 = CommandSetId::new("test",(1,2),0x1F3F9E7C72C86288);
        let mut cs1 = InterpLibRegister::new(&csi1);
        cs1.push(FakeDeserializer(v.clone(),5));
        cis.register(cs1).expect("f");
        //
        cis.adjust(&ccs.serialize()).expect("h");
        
        /* now, our opcodes should be flipped to match ccs */
        let mut context = InterpContext::new(&HashMap::new());
        cis.get_deserializer(5).expect("e").deserialize(5,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(5,*v.borrow());
        cis.get_deserializer(12).expect("e").deserialize(12,&vec![]).expect("f").execute(&mut context).expect("g");
        assert_eq!(6,*v.borrow());
    }

    fn age_check(compiler: (u32,u32), interpreter: (u32,u32)) -> bool {
        let mut ccs = CommandCompileSuite::new();

        let csi1 = CommandSetId::new("test",compiler,0x1F3D4E7C72C86288);
        let mut cs1 = InterpLibRegister::new(&csi1);
        cs1.push(NoopDeserializer(5));        
        let mut cs1 = CompLibRegister::new(&csi1,Some(cs1));
        cs1.push("test2",Some(5),fake_command("nc"));
        ccs.register(cs1).expect("a");

        let csi1 = CommandSetId::new("test",interpreter,0x1F3D4E7C72C86288);
        let mut mis = CommandInterpretSuite::new();
        let mut cs1 = InterpLibRegister::new(&csi1);
        cs1.push(NoopDeserializer(5));
        mis.register(cs1).expect("c");
        mis.adjust(&ccs.serialize()).is_ok()
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
        let mut ccs = CommandCompileSuite::new();

        let csi1 = CommandSetId::new("test",(1,1),0xB790000000000000);
        let cs1 = CompLibRegister::new(&csi1,None);
        ccs.register(cs1).expect("a");
        let csi1 = CommandSetId::new("test",(1,2),0xB790000000000000);
        let cs1 = CompLibRegister::new(&csi1,None);
        ccs.register(cs1).expect_err("b");
    }

    #[test]
    fn test_ok_multi_major() {
        let v : Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        let mut cis = CommandInterpretSuite::new();
        let csi2 = CommandSetId::new("test",(2,1),0x284E7C72C8628E94);
        let mut is2 = InterpLibRegister::new(&csi2);
        is2.push(FakeDeserializer(v.clone(),1));
        cis.register(is2).expect("c");
        let csi1 = CommandSetId::new("test",(1,1),0x43F9E7C72C8628C);
        let mut is1 = InterpLibRegister::new(&csi1);
        is1.push(FakeDeserializer(v.clone(),2));
        cis.register(is1).expect("c");
        let mut ccs = CommandCompileSuite::new();
        let csi1 = CommandSetId::new("test",(1,1),0x043F9E7C72C8628C);
        let mut is1 = InterpLibRegister::new(&csi1);
        is1.push(FakeDeserializer(v.clone(),2));
        let mut cs1 = CompLibRegister::new(&csi1,Some(is1));
        cs1.push("test1",Some(2),fake_command("c"));
        ccs.register(cs1).expect("a");

        cis.adjust(&ccs.serialize()).expect("d");
        let mut context = InterpContext::new(&HashMap::new());
        cis.get_deserializer(2).expect("e").deserialize(6,&vec![]).expect("f").execute(&mut context).expect("g");
        // TODO trace command in payload to replace Fake*
        assert_eq!(2,*v.borrow());
    }

    #[test]
    fn test_missing_set_bad_interp() {
        let mut ccs = CommandCompileSuite::new();

        let csi1 = CommandSetId::new("test",(1,1),0x1F3F9E7C72C86288);
        let is1 = InterpLibRegister::new(&csi1);
        let mut cs1 = CompLibRegister::new(&csi1,Some(is1));
        cs1.push("test1",Some(5),fake_command("c"));
        ccs.register(cs1).expect("a");

        let mut mis = CommandInterpretSuite::new();
        mis.adjust(&ccs.serialize()).expect_err("d");
    }

    #[test]
    fn test_missing_set_ok_compiler() {
        let ccs = CommandCompileSuite::new();
        let csi1 = CommandSetId::new("test",(1,1),0x1F3F9E7C72C86288);

        let mut mis = CommandInterpretSuite::new();
        let mut cs1 = InterpLibRegister::new(&csi1);
        cs1.push(NoopDeserializer(5));
        mis.register(cs1).expect("a");

        mis.adjust(&ccs.serialize()).expect("d");
    }

    #[test]
    fn test_dynamic_data() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_verbose(2);
        let cs = make_compiler_suite(&config).expect("y");
        let linker = CompilerLink::new(cs).expect("z");
        let data = linker.generate_dynamic_data(&config).expect("x");
        for (suite,data) in data.iter() {
            print!("command set {}\n",suite);
            cbor_serialize(&data).expect("a");
        }
    }
}
