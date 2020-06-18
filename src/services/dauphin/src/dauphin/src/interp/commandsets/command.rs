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
use crate::interp::context::InterpContext;
use crate::interp::CompilerLink;
use crate::model::{ Identifier, Register };
use crate::generate::{ Instruction, InstructionSuperType, PreImageContext };
use serde_cbor::Value as CborValue;

#[derive(Eq,PartialEq,Hash,Clone,Debug)]
pub enum CommandTrigger {
    Instruction(InstructionSuperType),
    Command(Identifier)
}

impl CommandTrigger {
    pub fn serialize(&self) -> CborValue {
        match self {
            CommandTrigger::Instruction(instr) =>
                CborValue::Array(vec![CborValue::Integer(0),instr.serialize()]),
            CommandTrigger::Command(ident) =>
                CborValue::Array(vec![CborValue::Integer(1),ident.serialize()])
        }
    }
}

pub enum PreImageOutcome {
    Skip,
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
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String>;
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> { Ok(CborValue::Null) }
}

pub trait Command {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String>;
    fn serialize(&self) -> Result<Vec<CborValue>,String>;
    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<bool,String> { Ok(false) }
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> { Err(format!("preimage_post must be overridden if simple_preimage returns true")) }
    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> { 
        Ok(if self.simple_preimage(context)? {
            self.execute(context.context())?;
            self.preimage_post(context)?
        } else {
            PreImageOutcome::Skip
        })
    }
    fn execution_time(&self) -> f64 { 0.1 }
}
