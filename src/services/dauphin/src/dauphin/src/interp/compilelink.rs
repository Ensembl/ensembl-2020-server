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

use std::collections::BTreeMap;
use std::rc::Rc;
use crate::cli::Config;
use crate::generate::{ Instruction, InstructionType };
use crate::interp::commandsets::{ Command, CommandSchema, CommandCompileSuite, CommandTrigger, CommandSuiteBuilder };
use serde_cbor::Value as CborValue;

pub(super) const VERSION : u32 = 0;

#[derive(Clone)]
pub struct CompilerLink {
    cs: Rc<CommandCompileSuite>
}

impl CompilerLink {
    pub fn new(cs: CommandSuiteBuilder) -> Result<CompilerLink,String> {
        Ok(CompilerLink {
            cs: Rc::new(cs.make_compile_suite()?)
        })
    }

    pub fn compile_instruction(&self, instr: &Instruction) -> Result<(u32,CommandSchema,Box<dyn Command>),String> {
        let (ct,opcode) = if let InstructionType::Call(identifier,_,_,_) = &instr.itype {
            self.cs.get_by_trigger(&CommandTrigger::Command(identifier.clone()))?
        } else {
            self.cs.get_by_trigger(&CommandTrigger::Instruction(instr.itype.supertype()?))?
        };
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

    pub fn serialize(&self, instrs: &[Instruction], config: &Config) -> Result<CborValue,String> {
        let cmds = instrs.iter().map(|x| self.compile_instruction(x)).collect::<Result<Vec<_>,_>>()?;
        let mut out = BTreeMap::new();
        let mut cmds_s = vec![];
        for (opcode,sch,cmd) in &cmds {
            self.serialize_command(&mut cmds_s,*opcode,sch,cmd)?;
        }
        out.insert(CborValue::Text("version".to_string()),CborValue::Integer(VERSION as i128));
        out.insert(CborValue::Text("suite".to_string()),self.cs.serialize().clone());
        out.insert(CborValue::Text("program".to_string()),CborValue::Array(cmds_s));
        if config.get_generate_debug() {
            out.insert(CborValue::Text("instructions".to_string()),CborValue::Array(instrs.iter().map(|x| self.serialize_instruction(x)).collect::<Vec<_>>()));
        }
        Ok(CborValue::Map(out))
    }
}