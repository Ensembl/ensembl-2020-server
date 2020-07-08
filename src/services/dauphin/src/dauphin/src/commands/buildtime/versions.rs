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
use crate::model::{ Register, Identifier };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, InterpContext, PreImageOutcome };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use crate::interp::InterpValue;
use crate::generate::PreImageContext;
use crate::resolver::Resolver;
use ini::Ini;

pub struct VersionCommandType();

impl CommandType for VersionCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","get_version"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            let mut major = None;
            let mut minor = None;
            for (path,vr) in sig[0].iter() {
                if let Some(name) = path.get_name() {
                    if name.len() == 1 {
                        // TODO utility methods
                        let name = &name[0];
                        if name == "major" { major = Some(&it.regs[vr.data_pos()]) }
                        if name == "minor" { minor = Some(&it.regs[vr.data_pos()]) }
                    }
                }
            }
            if major.is_none() || minor.is_none() {
                return Err(format!("buildtime:version signature issue"))
            }
            let libname = it.regs[sig[1].iter().next().as_ref().unwrap().1.data_pos()];
            Ok(Box::new(VersionCommand(*major.unwrap(),*minor.unwrap(),libname)))
        } else {
            Err(format!("buildtime::version cannot be built"))
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Err(format!("buildtime::version can only be executed at compile time"))
    }
}

pub struct VersionCommand(Register,Register,Register);

impl Command for VersionCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Err(format!("buildtime::version can only be executed at compile time"))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Err(format!("buildtime::version can only be executed at compile time"))
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        if context.is_reg_valid(&self.2) {
            let suite = context.linker().get_suite().get_set_ids();
            let versions : HashMap<_,_> = suite.iter().map(|x| (x.name().to_string(),x.version())).collect();
            let mut majors = vec![];
            let mut minors = vec![];
            for name in context.context().registers().get_strings(&self.2)?.iter() {
                let (major,minor) = if let Some((major,minor)) = versions.get(name) {
                    (*major as usize,*minor as usize)
                } else {
                    (0,0)
                };
                majors.push(major);
                minors.push(minor);
            }
            context.context_mut().registers_mut().write(&self.0,InterpValue::Indexes(majors));
            context.context_mut().registers_mut().write(&self.1,InterpValue::Indexes(minors));
            Ok(PreImageOutcome::Constant(vec![self.0,self.1]))
        } else {
            Err(format!("buildtime::version needs key to be known at build time"))
        }
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
    fn versions_smoke() {
        let mut config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:buildtime/versions").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}
