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

use std::fmt;
use crate::cli::Config;
use crate::command::{ Instruction, InstructionSuperType, CompilerLink };
use crate::model::PreImageContext;
use dauphin_interp::util::cbor::{ cbor_array, cbor_int };
use dauphin_interp::command::{ Identifier, InterpCommand };
use dauphin_interp::runtime::Register;
use serde_cbor::Value as CborValue;

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub enum CommandTrigger {
    Instruction(InstructionSuperType),
    Command(Identifier)
}

impl CommandTrigger {
    pub fn deserialize(value: &CborValue) -> Result<CommandTrigger,String> {
        let data = cbor_array(value,2,false)?;
        Ok(match cbor_int(&data[0],None)? {
            0 => CommandTrigger::Instruction(InstructionSuperType::deserialize(&data[1])?),
            _ => CommandTrigger::Command(Identifier::deserialize(&data[1])?)
        })
    }

    pub fn serialize(&self) -> CborValue {
        match self {
            CommandTrigger::Instruction(instr) =>
                CborValue::Array(vec![CborValue::Integer(0),instr.serialize()]),
            CommandTrigger::Command(ident) =>
                CborValue::Array(vec![CborValue::Integer(1),ident.serialize()])
        }
    }
}

pub enum PreImagePrepare {
    Keep(Vec<(Register,usize)>),
    Replace
}

pub enum PreImageOutcome {
    Skip(Vec<(Register,usize)>),
    Constant(Vec<Register>),
    Replace(Vec<Instruction>)
}

impl fmt::Display for CommandTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandTrigger::Command(cmd) => write!(f,"{}",cmd),
            CommandTrigger::Instruction(instr) => write!(f,"builtin({:?})",instr)
        }
    }
}

pub struct CommandSchema {
    pub values: usize,
    pub trigger: CommandTrigger
}

pub trait CommandType {
    fn get_schema(&self) -> CommandSchema;
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String>;
    fn generate_dynamic_data(&self, _linker: &CompilerLink, _config: &Config) -> Result<CborValue,String> { Ok(CborValue::Null) }
    fn use_dynamic_data(&mut self, _value: &CborValue) -> Result<(),String> { Ok(()) }
}

pub trait Command {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String>;
    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<PreImagePrepare,String> { Ok(PreImagePrepare::Keep(vec![])) }
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> { Err(format!("preimage_post must be overridden if simple_preimage returns true")) }
    fn preimage(&self, context: &mut PreImageContext, ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        Ok(match self.simple_preimage(context)? {
            PreImagePrepare::Replace => {
                let ic = ic.ok_or_else(|| format!("cannot compile-side run despite being asked to!"))?;
                ic.execute(context.context_mut())?;
                self.preimage_post(context)?    
            },
            PreImagePrepare::Keep(sizes) => {
                PreImageOutcome::Skip(sizes)
            }
        })
    }
    fn execution_time(&self, _context: &PreImageContext) -> f64 { 1. }
}
