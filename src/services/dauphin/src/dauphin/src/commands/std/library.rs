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

use crate::interp::{
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, PreImagePrepare, CompLibRegister
};
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use super::numops::{ library_numops_commands, library_numops_commands_interp };
use super::eq::{ library_eq_command, library_eq_command_interp };
use super::assign::{ library_assign_commands, library_assign_commands_interp };
use super::print::{ PrintCommandType, PrintDeserializer };
use super::vector::{ library_vector_commands, library_vector_commands_interp };
use dauphin_interp_common::common::{ InterpCommand, Register, RegisterSignature, Identifier, CommandDeserializer, NoopDeserializer, CommandSetId };
use dauphin_interp_common::interp::{ InterpLibRegister, Stream, InterpContext };

pub fn std_id() -> CommandSetId {
    CommandSetId::new("std",(0,0),0x8A07AE1254D6E44B)
}

pub(super) fn std(name: &str) -> Identifier {
    Identifier::new("std",name)
}

pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
    let p = context.payload("std","stream")?;
    Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
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
        Ok(Some(vec![CborValue::Array(self.1.iter().map(|x| x.serialize()).collect()),self.0.serialize(false,false)?]))
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

pub struct AssertDeserializer();

impl CommandDeserializer for AssertDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((4,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(AssertInterpCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?)))
    }
}

pub struct AssertInterpCommand(Register,Register);

impl InterpCommand for AssertInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let a = &registers.get_boolean(&self.0)?;
        let b = &registers.get_boolean(&self.1)?;
        for i in 0..a.len() {
            if a[i] != b[i%b.len()] {
                return Err(format!("assertion failed index={}!",i));
            }
        }
        Ok(())
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
    /* 2,3 are free */
    set.push("len",None,LenCommandType());
    set.push("assert",Some(4),AssertCommandType());
    set.push("alienate",Some(13),AlienateCommandType());
    set.push("print",Some(14),PrintCommandType());
    set.add_header("std",include_str!("header.dp"));
    library_numops_commands(&mut set)?;
    library_assign_commands(&mut set)?;
    library_vector_commands(&mut set)?;
    set.dynamic_data(include_bytes!("std-0.0.ddd"));
    Ok(set)
}

pub fn make_std_interp() -> Result<InterpLibRegister,String> {
    let mut set = InterpLibRegister::new(&std_id());
    library_eq_command_interp(&mut set)?;
    set.push(AssertDeserializer());
    set.push(NoopDeserializer(13));
    set.push(PrintDeserializer());
    library_numops_commands_interp(&mut set)?;
    library_assign_commands_interp(&mut set)?;
    library_vector_commands_interp(&mut set)?;
    Ok(set)
}
