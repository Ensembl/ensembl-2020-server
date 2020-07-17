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

use dauphin_compile::command::{
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, PreImagePrepare, CompLibRegister, Instruction, InstructionType
};
use dauphin_compile::model::PreImageContext;
use dauphin_interp::command::{ InterpCommand, CommandSetId, Identifier };
use dauphin_interp::types::{ RegisterSignature };
use dauphin_interp::runtime::{ Register };
use serde_cbor::Value as CborValue;
use super::numops::{ library_numops_commands };
use super::eq::{ library_eq_command };
use super::assign::{ library_assign_commands };
use super::print::{ PrintCommandType, FormatCommandType };
use super::vector::{ library_vector_commands };
use crate::make_std_interp;

pub fn std_id() -> CommandSetId {
    CommandSetId::new("std",(0,0),0xDB806BE64887FAA9)
}

pub(super) fn std(name: &str) -> Identifier {
    Identifier::new("std",name)
}

pub struct LenCommandType();

impl CommandType for LenCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("len"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(LenCommand(sig.clone(),it.regs.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
}

pub struct LenCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for LenCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![CborValue::Array(self.1.iter().map(|x| x.serialize()).collect()),self.0.serialize(false)?]))
    }

    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        if let Some((_,ass)) = &self.0[1].iter().next() {
            let reg = ass.length_pos(ass.depth()-1)?;
            if context.is_reg_valid(&self.1[reg]) && !context.is_last() {
                /* can execute now */
                context.context_mut().registers_mut().copy(&self.1[0],&self.1[reg])?;
                return Ok(PreImageOutcome::Constant(vec![self.1[0]]));
            } else {
                /* replace */
                return Ok(PreImageOutcome::Replace(vec![
                    Instruction::new(InstructionType::Copy,vec![self.1[0].clone(),self.1[reg].clone()])
                ]))
            }
        }
        /* should never happen! */
        Err(format!("cannot preimage length command"))
    }
}

pub struct AssertCommandType();

impl CommandType for AssertCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("assert"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(AssertCommand(it.regs[0],it.regs[1])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct AssertCommand(Register,Register);

impl Command for AssertCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(if context.is_reg_valid(&self.0) && context.is_reg_valid(&self.1) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.0) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Replace(vec![]))
    }
}

pub struct AlienateCommandType();

impl CommandType for AlienateCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(std("alienate"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(AlienateCommand(it.regs.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct AlienateCommand(pub(crate) Vec<Register>);

impl Command for AlienateCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }
    
    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        for reg in self.0.iter() {
            context.set_reg_invalid(reg);
            context.set_reg_size(reg,None);
        }
        Ok(PreImageOutcome::Skip(vec![]))
    }
}

pub fn make_std() -> Result<CompLibRegister,String> {
    let mut set = CompLibRegister::new(&std_id(),Some(make_std_interp()?));
    library_eq_command(&mut set)?;
    /* 3 is free */
    set.push("len",None,LenCommandType());
    set.push("assert",Some(4),AssertCommandType());
    set.push("alienate",Some(13),AlienateCommandType());
    set.push("print",Some(14),PrintCommandType());
    set.push("format",Some(2),FormatCommandType());
    set.add_header("std",include_str!("header.dp"));
    library_numops_commands(&mut set)?;
    library_assign_commands(&mut set)?;
    library_vector_commands(&mut set)?;
    set.dynamic_data(include_bytes!("std-0.0.ddd"));
    Ok(set)
}
