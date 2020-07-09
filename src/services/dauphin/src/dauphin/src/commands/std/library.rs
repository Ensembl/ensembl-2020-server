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
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier, cbor_make_map, ComplexRegisters };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents, PreImageOutcome, Stream, PreImagePrepare, InterpValue };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use super::numops::library_numops_commands;
use super::eq::library_eq_command;
use super::assign::library_assign_commands;
use super::print::PrintCommandType;
use super::vector::library_vector_commands;
use crate::cli::Config;
use crate::typeinf::{ MemberMode, BaseType };
use crate::interp::{ CompilerLink, TimeTrialCommandType, trial_write, trial_signature, TimeTrial };

pub fn std_id() -> CommandSetId {
    CommandSetId::new("std",(0,0),0xAE0FBDF35D05BAE8)
}

pub(super) fn std(name: &str) -> Identifier {
    Identifier::new("std",name)
}

pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
    let p = context.payload("std","stream")?;
    Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
}

struct LenTimeTrial();

impl TimeTrialCommandType for LenTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        trial_write(context,1,1,|_| 0);
        trial_write(context,2,1,|_| t);
        trial_write(context,3,t,|x| x*10);
        trial_write(context,4,t,|_| 10);
        trial_write(context,5,t*10,|x| x);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sig = trial_signature(&vec![(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,2,BaseType::NumberType)]);
        let regs : Vec<Register> = (0..6).map(|x| Register(x)).collect();
        Ok(Box::new(LenCommand(sig,regs)))
    }
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
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<Vec<_>,_>>()?;
        Ok(Box::new(LenCommand(RegisterSignature::deserialize(&value[1],false,false)?,regs)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&LenTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }
}

pub struct LenCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for LenCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        if let Some((_,ass)) = &self.0[1].iter().next() {
            let reg = ass.length_pos(ass.depth()-1)?;
            registers.copy(&self.1[0],&self.1[reg])?;
            Ok(())
        } else {
            Err("len on non-list".to_string())
        }
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![CborValue::Array(self.1.iter().map(|x| x.serialize()).collect()),self.0.serialize(false,false)?]))
    }

    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        if let Some((_,ass)) = &self.0[1].iter().next() {
            let reg = ass.length_pos(ass.depth()-1)?;
            if context.is_reg_valid(&self.1[reg]) && !context.is_last() {
                /* can execute now */
                self.execute(context.context_mut())?;
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
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AssertCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?)))
    }
}

pub struct AssertCommand(pub(crate) Register, pub(crate) Register);

impl Command for AssertCommand {
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
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AlienateCommand(vec![])))
    }
}

pub struct AlienateCommand(pub(crate) Vec<Register>);

impl Command for AlienateCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }
    
    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        for reg in self.0.iter() {
            context.set_reg_invalid(reg);
            context.set_reg_size(reg,None);
        }
        Ok(PreImageOutcome::Skip(vec![]))
    }
}

pub fn make_library() -> Result<CommandSet,String> {
    let mut set = CommandSet::new(&std_id(),false);
    library_eq_command(&mut set)?;
    /* 2,3 are free */
    set.push("len",1,LenCommandType())?;
    set.push("assert",4,AssertCommandType())?;
    set.push("alienate",13,AlienateCommandType())?;
    set.push("print",14,PrintCommandType())?;
    set.add_header("std",include_str!("header.dp"));
    library_numops_commands(&mut set)?;
    library_assign_commands(&mut set)?;
    library_vector_commands(&mut set)?;
    set.load_dynamic_data(include_bytes!("std-0.0.ddd"))?;
    Ok(set)
}
