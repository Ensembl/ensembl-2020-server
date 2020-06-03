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

use crate::model::{ Register, Identifier };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, InterpContext, PreImageOutcome };
use crate::generate::Instruction;
use serde_cbor::Value as CborValue;
use crate::interp::InterpValue;
use crate::generate::PreImageContext;
use crate::resolver::Resolver;
use ini::Ini;

pub struct LoadIniCommandType();

impl CommandType for LoadIniCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 4,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","load_ini"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(LoadIniCommand(it.regs[0],it.regs[1],it.regs[2],it.regs[3])))
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }
}

pub struct LoadIniCommand(Register,Register,Register,Register);

fn load_ini(resolver: &Resolver, filename: &str, section: &str, key: &str) -> Result<String,String> {
    let data = resolver.resolve(filename)?.0.to_string();
    let ini_file = Ini::load_from_str(&data).map_err(|e| format!("Cannot parse {}: {}",filename,e))?;
    let section = if section == "" { None } else { Some(section.to_string()) };
    let ini_section = ini_file.section(section).ok_or_else(|| format!("No such section"))?;
    let value = ini_section.get(key).ok_or_else(|| format!("No such key {}",key))?.to_string();
    Ok(value)
}

fn load_inis(resolver: &Resolver, filenames: &[String], sections: &[String], keys: &[String]) -> Result<Vec<String>,String> {
    let mut out = vec![];
    let sec_len = sections.len();
    let fn_len = filenames.len();
    for (i,key) in keys.iter().enumerate() {
        out.push(load_ini(resolver,&filenames[i%fn_len],&sections[i%sec_len],key)?);
    }
    Ok(out)
}

impl Command for LoadIniCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        if context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2) && context.get_reg_valid(&self.3) {
            let (filenames,sections,keys) = {
                let regs = context.context().registers();
                (
                    &regs.get_strings(&self.1)?,
                    &regs.get_strings(&self.2)?,
                    &regs.get_strings(&self.3)?
                )
            };
            let out = load_inis(context.resolver(),filenames,sections,keys)?;
            context.context().registers().write(&self.0,InterpValue::Strings(out));
            context.set_reg_valid(&self.0,true);
            Ok(PreImageOutcome::Constant(vec![self.0]))
        } else {
            Err(format!("buildtime::load_ini needs all arguments to be known at build time 1st/2nd/3rd-arg-known={}/{}/{}",
                            context.get_reg_valid(&self.1),context.get_reg_valid(&self.2),context.get_reg_valid(&self.3)))
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
    fn load_ini_smoke() {
        let mut config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:buildtime/load_ini").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}
