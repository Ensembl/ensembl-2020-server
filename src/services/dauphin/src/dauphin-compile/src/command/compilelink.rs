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
use crate::command::{ Instruction, InstructionType, CommandCompileSuite, Command, CommandSchema, CommandTrigger };
use dauphin_interp::runtime::{ InterpContext, PayloadFactory };
use dauphin_interp::command::{ CommandSetId, InterpCommand };
use serde_cbor::Value as CborValue;

pub(super) const VERSION : u32 = 0;

#[derive(Clone)]
pub struct CompilerLink {
    cs: Rc<CommandCompileSuite>,
    headers: HashMap<String,String>,
    programs: BTreeMap<CborValue,CborValue>,
    payloads: HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>
}

impl CompilerLink {
    pub fn new(cs: CommandCompileSuite) -> Result<CompilerLink,String> {
        let payloads = cs.copy_payloads().clone();
        let headers = cs.get_headers().clone();
        Ok(CompilerLink {
            cs: Rc::new(cs),
            headers: headers.clone(),
            programs: BTreeMap::new(),
            payloads
        })
    }

    pub fn generate_dynamic_data(&self, config: &Config) -> Result<HashMap<CommandSetId,CborValue>,String> {
        Ok(self.cs.generate_dynamic_data(&self,config)?)
    }

    pub fn get_suite(&self) -> &Rc<CommandCompileSuite> { &self.cs }
    pub fn get_headers(&self) -> &HashMap<String,String> { &self.headers }

    fn instruction_to_trigger(&self, instr: &Instruction) -> Result<CommandTrigger,String> {
        Ok(if let InstructionType::Call(identifier,_,_,_) = &instr.itype {
            CommandTrigger::Command(identifier.clone())
        } else {
            CommandTrigger::Instruction(instr.itype.supertype()?)
        })
    }

    pub fn instruction_to_command(&self, instr: &Instruction) -> Result<(CommandSchema,Box<dyn Command>),String> {
        let ct = self.cs.get_command_by_trigger(&self.instruction_to_trigger(instr)?)?;
        Ok((ct.get_schema(),ct.from_instruction(instr)?))
    }

    pub fn instruction_to_interp_command(&self, instr: &Instruction) -> Result<Option<Box<dyn InterpCommand>>,String> {
        if let Some(ds) = self.cs.get_deserializer_by_trigger(&self.instruction_to_trigger(instr)?)? {
            if let Some(opcode) = ds.get_opcode_len()? {
                let (_sch,command) = self.instruction_to_command(instr)?;
                if let Some(data) = command.serialize()? {
                    let data_ref : Vec<&CborValue> = data.iter().collect();
                    return Ok(Some(ds.deserialize(opcode.0,&data_ref)?));
                }
            }
        }
        return Ok(None)
    }

    pub fn instruction_to_opcode(&self, instr: &Instruction) -> Result<Option<u32>,String> {
        let opcode = if let InstructionType::Call(identifier,_,_,_) = &instr.itype {
            self.cs.get_opcode_by_trigger(&CommandTrigger::Command(identifier.clone()))?
        } else {
            self.cs.get_opcode_by_trigger(&CommandTrigger::Instruction(instr.itype.supertype()?))?
        };
        Ok(opcode)
    }

    fn serialize_command(&self, out: &mut Vec<CborValue>, opcode: u32, schema: &CommandSchema, command: &Box<dyn Command>) -> Result<bool,String> {
        if let Some(mut data) = command.serialize()? {
            if data.len() != schema.values {
                return Err(format!("serialization of {} returned {} values, expected {}",schema.trigger,data.len(),schema.values));
            }
            out.push(CborValue::Integer(opcode as i128));
            out.append(&mut data);
            Ok(true)
        } else {
            Ok(false)
        }
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
        let mut cmds_s = vec![];
        let mut symbols = vec![];
        for (i,instr) in instrs.iter().enumerate() {
            if let Some(opcode) = self.instruction_to_opcode(instr)? {
                let (sch,cmd) = self.instruction_to_command(instr)?;
                let gen = self.serialize_command(&mut cmds_s,opcode,&sch,&cmd)?;
                if gen && config.get_generate_debug() {
                    symbols.push(self.serialize_instruction(&instrs[i]));
                }
            }
        }
        let mut program = BTreeMap::new();
        program.insert(CborValue::Text("cmds".to_string()),CborValue::Array(cmds_s));
        if config.get_generate_debug() {
            program.insert(CborValue::Text("symbols".to_string()),CborValue::Array(symbols));
        }
        Ok(CborValue::Map(program))
    }

    pub fn serialize(&self, _config: &Config) -> Result<CborValue,String> {
        let mut out = BTreeMap::new();
        out.insert(CborValue::Text("version".to_string()),CborValue::Integer(VERSION as i128));
        out.insert(CborValue::Text("suite".to_string()),self.cs.serialize().clone());
        out.insert(CborValue::Text("programs".to_string()),CborValue::Map(self.programs.clone()));
        Ok(CborValue::Map(out))
    }

    pub fn new_context(&self) -> InterpContext {
        InterpContext::new(&self.payloads)
    }
}
