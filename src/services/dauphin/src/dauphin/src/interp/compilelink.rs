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

use std::collections::{ BTreeMap, HashMap };
use std::rc::Rc;
use crate::cli::Config;
use crate::generate::{ Instruction, InstructionType };
use crate::interp::commandsets::{ Command, CommandSchema, CommandCompileSuite, CommandTrigger, LibrarySuiteBuilder };
use serde_cbor::Value as CborValue;

pub(super) const VERSION : u32 = 0;

#[derive(Clone)]
pub struct CompilerLink {
    cs: Rc<CommandCompileSuite>,
    headers: HashMap<String,String>,
    programs: BTreeMap<CborValue,CborValue>
}

impl CompilerLink {
    pub fn new(cs: LibrarySuiteBuilder) -> Result<CompilerLink,String> {
        let headers = cs.get_headers().clone();
        Ok(CompilerLink {
            cs: Rc::new(cs.make_compile_suite()?),
            headers,
            programs: BTreeMap::new()
        })
    }

    pub fn get_headers(&self) -> &HashMap<String,String> { &self.headers }

    pub fn compile_instruction(&self, instr: &Instruction, compile_side: bool) -> Result<(u32,CommandSchema,Box<dyn Command>),String> {
        let mut name = "*anon*".to_string();
        let (ct,opcode,compile_only) = if let InstructionType::Call(identifier,_,_,_) = &instr.itype {
            name = identifier.to_string();
            self.cs.get_by_trigger(&CommandTrigger::Command(identifier.clone()))?
        } else {
            self.cs.get_by_trigger(&CommandTrigger::Instruction(instr.itype.supertype()?))?
        };
        if compile_only && !compile_side {
            return Err(format!("{} is a compile-side only command",name));
        }
        Ok((opcode,ct.get_schema(),ct.from_instruction(instr)?))
    }

    fn serialize_command(&self, out: &mut Vec<CborValue>, opcode: u32, schema: &CommandSchema, command: &Box<dyn Command>) -> Result<(),String> {
        let mut data = command.serialize()?;
        if data.len() != schema.values {
            return Err(format!("serialization of {} returned {} values, expected {}",schema.trigger,data.len(),schema.values));
        }
        out.push(CborValue::Integer(opcode as i128));
        out.append(&mut data);
        Ok(())
    }

    fn serialize_instruction(&self, instruction: &Instruction) -> CborValue {
        CborValue::Array(vec![
            CborValue::Text(format!("{:?}",instruction)),
            CborValue::Array(
                instruction.regs.iter().map(|x| x.serialize()).collect()
            )
        ])
    }

    pub fn add(&mut self, name: &str, instrs: &[Instruction], config: &Config) -> Result<(),String> {
        self.programs.insert(CborValue::Text(name.to_string()),self.serialize_program(instrs,config)?);
        Ok(())
    }

    fn serialize_program(&self, instrs: &[Instruction], config: &Config) -> Result<CborValue,String> {
        let cmds = instrs.iter().map(|x| self.compile_instruction(x,false)).collect::<Result<Vec<_>,_>>()?;
        let mut cmds_s = vec![];
        for (opcode,sch,cmd) in &cmds {
            self.serialize_command(&mut cmds_s,*opcode,sch,cmd)?;
        }
        let mut program = BTreeMap::new();
        program.insert(CborValue::Text("cmds".to_string()),CborValue::Array(cmds_s));
        if config.get_generate_debug() {
            let symbols = instrs.iter().map(|x| self.serialize_instruction(x)).collect::<Vec<_>>();
            program.insert(CborValue::Text("symbols".to_string()),CborValue::Array(symbols));
        }
        Ok(CborValue::Map(program))
    }

    pub fn serialize(&mut self, config: &Config) -> Result<CborValue,String> {
        let mut out = BTreeMap::new();
        out.insert(CborValue::Text("version".to_string()),CborValue::Integer(VERSION as i128));
        out.insert(CborValue::Text("suite".to_string()),self.cs.serialize().clone());
        out.insert(CborValue::Text("programs".to_string()),CborValue::Map(self.programs.clone()));
        Ok(CborValue::Map(out))
    }
}

#[cfg(test)]
mod test {
    use crate::cli::Config;
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::resolver::Resolver;
    use crate::generate::generate;
    use crate::interp::{ mini_interp_run, CompilerLink, xxx_test_config, make_librarysuite_builder };
    use crate::interp::context::InterpContext;

    fn make_program(linker: &mut CompilerLink, resolver: &Resolver, config: &Config, name: &str, path: &str) -> Result<(),String> {
        let mut lexer = Lexer::new(&resolver);
        lexer.import(path).expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        linker.add(name,&instrs,config)?;
        Ok(())
    }

    #[test]
    fn test_multi_program() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_verbose(2);
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        make_program(&mut linker,&resolver,&config,"prog1","search:codegen/multiprog1").expect("cannot build prog1");
        make_program(&mut linker,&resolver,&config,"prog2","search:codegen/multiprog2").expect("cannot build prog2");
        make_program(&mut linker,&resolver,&config,"prog3","search:codegen/multiprog3").expect("cannot build prog3");
        let program = linker.serialize(&config).expect("serialize");
        let mut ic = InterpContext::new();
        let (_,a) = mini_interp_run(&program,&mut ic,&config,"prog2").expect("A");
        let (_,b) = mini_interp_run(&program,&mut ic,&config,"prog1").expect("B");
        assert_eq!(vec!["prog2"],a);
    }
}
