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

use crate::model::{ Register, Identifier, RegisterSignature };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, InterpContext, PreImageOutcome };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use crate::interp::InterpValue;
use crate::generate::PreImageContext;

pub struct DumpSigCommandType();

fn sig_string(sig: &RegisterSignature) -> String {
    format!("{:?}",sig[1])
}

impl CommandType for DumpSigCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","dump_sig"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(DumpSigCommand(it.regs[0],sig_string(sig))))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Err(format!("buildtime::dump_sig can only be executed at compile time"))
    }
}

pub struct DumpSigCommand(Register,String);

impl Command for DumpSigCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Err(format!("buildtime::dump_sig can only be executed at compile time"))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.context_mut().registers_mut().write(&self.0,InterpValue::Strings(vec![self.1.to_string()]));
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_librarysuite_builder };

    #[test]
    fn dump_sig() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:buildtime/dump_sig").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        assert!(strings[0].contains("[0, 1]"));
        assert!(strings[1].contains("[1, 0]"));
        assert!(strings[2].contains("[1, 1]"));
    }
}
