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

use dauphin_compile_common::command::{ Command, CommandSchema, CommandType, CommandTrigger };
use dauphin_compile_common::model::{ Instruction, InstructionType };
use dauphin_interp_common::common::{ Register, RegisterSignature, Identifier };
use serde_cbor::Value as CborValue;

pub struct FormatCommandType();

impl CommandType for FormatCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(Identifier::new("std","format"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(FormatCommand(it.regs.to_vec(),sig.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct FormatCommand(Vec<Register>,RegisterSignature);

impl Command for FormatCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.0.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![regs,self.1.serialize(true)?]))
    }
}

pub struct PrintCommandType();

impl CommandType for PrintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(Identifier::new("std","print"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(PrintCommand(it.regs[0])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct PrintCommand(Register);

impl Command for PrintCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize()]))
    }    
}

#[cfg(test)]
mod test {
    use crate::test::{ mini_interp, xxx_test_config, make_compiler_suite, compile };
    use dauphin_compile_common::model::CompilerLink;

    #[test]
    fn print_smoke() {
        let config = xxx_test_config();
        let strings = compile(&config,"search:std/print").expect("a");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(&vec![
            "[print::test3 { A: [[1, 1], [1, 2, 3], [4, 5, 6], [7, 8, 9], [1, 1]], B: [] }, print::test3 { A: [[7], [6], [5]], B: [[4]] }]",
            "[buildtime::version { major: 0, minor: 1 }, buildtime::version { major: 0, minor: 0 }, buildtime::version { major: 0, minor: 0 }]",
            "[print::test { x: [false, true] }, print::test { x: [true, false] }]",
            "[print::test2:A [true, true], print::test2:B [[0], [1, 2, 3]], print::test2:C false, print::test2:A [false]]",
            "1", "2", "3",
            "\'4241030040\'"
        ].iter().map(|x| x.to_string()).collect::<Vec<_>>(),&strings);
    }
}
