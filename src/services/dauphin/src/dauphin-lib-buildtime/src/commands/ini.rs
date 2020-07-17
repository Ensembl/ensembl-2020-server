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

use dauphin_interp::command::{ Identifier, InterpCommand };
use dauphin_interp::runtime::{ InterpValue, Register };
use dauphin_compile::command::{ Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, Instruction };
use dauphin_compile::model::{ PreImageContext };
use serde_cbor::Value as CborValue;
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
}

pub struct LoadIniCommand(Register,Register,Register,Register);

fn load_ini(context: &PreImageContext, filename: &str, section: &str, key: &str) -> Result<String,String> {
    let data = context.resolve(filename)?;
    let ini_file = Ini::load_from_str(&data).map_err(|e| format!("Cannot parse {}: {}",filename,e))?;
    let section = if section == "" { None } else { Some(section.to_string()) };
    let ini_section = ini_file.section(section).ok_or_else(|| format!("No such section"))?;
    let value = ini_section.get(key).ok_or_else(|| format!("No such key {}",key))?.to_string();
    Ok(value)
}

fn load_inis(context: &PreImageContext, filenames: &[String], sections: &[String], keys: &[String]) -> Result<Vec<String>,String> {
    let mut out = vec![];
    let sec_len = sections.len();
    let fn_len = filenames.len();
    for (i,key) in keys.iter().enumerate() {
        out.push(load_ini(context,&filenames[i%fn_len],&sections[i%sec_len],key)?);
    }
    Ok(out)
}

impl Command for LoadIniCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Err(format!("buildtime::load_ini can only be executed at compile time"))
    }

    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && context.is_reg_valid(&self.3) {
            let (filenames,sections,keys) = {
                let regs = context.context_mut().registers_mut();
                (
                    &regs.get_strings(&self.1)?,
                    &regs.get_strings(&self.2)?,
                    &regs.get_strings(&self.3)?
                )
            };
            let out = load_inis(context,filenames,sections,keys)?;
            context.context_mut().registers_mut().write(&self.0,InterpValue::Strings(out));
            Ok(PreImageOutcome::Constant(vec![self.0]))
        } else {
            Err(format!("buildtime::load_ini needs all arguments to be known at build time 1st/2nd/3rd-arg-known={}/{}/{}",
                            context.is_reg_valid(&self.1),context.is_reg_valid(&self.2),context.is_reg_valid(&self.3)))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test::{ compile, xxx_test_config };
    
    #[test]
    fn load_ini_smoke() {
        let mut config = xxx_test_config();
        config.add_define(("yes".to_string(),"".to_string()));
        config.add_define(("hello".to_string(),"world".to_string()));
        let strings = compile(&config,"search:buildtime/load_ini").expect("a");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}
