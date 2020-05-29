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

use crate::interp::InterpNatural;
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents, PreImageOutcome };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use crate::interp::InterpValue;
use crate::generate::{ InstructionSuperType, PreImageContext };
use crate::resolver::Resolver;
use ini::Ini;

pub struct LoadIniCommandType();

impl CommandType for LoadIniCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","load_ini"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(LoadIniCommand(it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }
}

pub struct LoadIniCommand(pub(crate) Register, pub(crate) Register, pub(crate) Register);

fn load_ini(resolver: &Resolver, filename: &str, key: &str) -> Result<String,String> {
    let data = resolver.resolve(filename)?.0.to_string();
    let ini_file = Ini::load_from_str(&data).map_err(|e| format!("Cannot parse {}: {}",filename,e))?;
    let section: Option<String> = None;
    let ini_section = ini_file.section(section).ok_or_else(|| format!("No such section"))?;
    let value = ini_section.get(key).ok_or_else(|| format!("No such key {}",key))?.to_string();
    Ok(value)
}

fn load_inis(resolver: &Resolver, filenames: &[String], keys: &[String]) -> Result<Vec<String>,String> {
    let mut out = vec![];
    let fn_len = filenames.len();
    for (i,key) in keys.iter().enumerate() {
        out.push(load_ini(resolver,&filenames[i%fn_len],key)?);
    }
    Ok(out)
}

impl Command for LoadIniCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        if context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2) {
            let (filenames,keys) = {
                let run_context = context.context();
                let regs = run_context.registers();
                (
                    &regs.get_strings(&self.1)?,
                    &regs.get_strings(&self.2)?
                )
            };
            let out = load_inis(context.resolver(),filenames,&keys)?;
            context.context().registers().write(&self.0,InterpValue::Strings(out));
            context.set_reg_valid(&self.0,true);
            Ok(PreImageOutcome::Constant(vec![self.0]))
        } else {
            Err(format!("buildtime::load_ini needs all arguments to be known at build time 1st-arg-known={} 2nd-arg-known={}",context.get_reg_valid(&self.1),context.get_reg_valid(&self.2)))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::test_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, xxx_compiler_link, xxx_test_config };

    #[test]
    fn load_ini_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(&resolver);
        lexer.import("test:buildtime/load_ini.dp").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let config = xxx_test_config();
        let linker = xxx_compiler_link().expect("y");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&linker,&config).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}
