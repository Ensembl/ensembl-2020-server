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

use dauphin_interp_common::common::{ 
    Register, RegisterSignature, Identifier
};
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;

pub struct PrintCommandType();

impl CommandType for PrintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(Identifier::new("std","print"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(PrintCommand(it.regs.clone(),sig.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct PrintCommand(Vec<Register>,RegisterSignature);

impl Command for PrintCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.0.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![regs,self.1.serialize(true,true)?]))
    }    
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_compiler_suite };

    #[test]
    fn print_smoke() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:std/print").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
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
