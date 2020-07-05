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

use std::collections::HashSet;
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier, cbor_make_map, ComplexRegisters };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents, PreImageOutcome, Stream, PreImagePrepare, InterpValue };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use crate::typeinf::MemberMode;
use serde_cbor::Value as CborValue;

fn hint_reg(sig: &ComplexRegisters, regs: &[Register], incl_length: bool) -> Result<HashSet<Register>,String> {
    let mut out = HashSet::new();
    for (_,vr) in sig.iter() {
        if vr.depth() > 0 {
            out.insert(regs[vr.offset_pos(vr.depth()-1)?]);
            if incl_length {
                out.insert(regs[vr.length_pos(vr.depth()-1)?]);
            }
        } else {
            out.insert(regs[vr.data_pos()]);
        }
    }
    Ok(out)
}

pub struct GetSizeHintCommandType();

impl CommandType for GetSizeHintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","get_size_hint"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(GetSizeHintCommand(it.regs[0].clone(),hint_reg(&sig[1],&it.regs,false)?.iter().cloned().collect())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Err(format!("cannot deseriailize size hints"))
    }
}

pub struct GetSizeHintCommand(Register,Vec<Register>);

impl Command for GetSizeHintCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Err(format!("cannot execute size hints"))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Err(format!("cannot seriailize size hints"))
    }
    
    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        let mut out = vec![];
        for reg in self.1.iter() {
            out.push(context.get_reg_size(reg).unwrap_or(1000000000));
        }
        context.context_mut().registers_mut().write(&self.0,InterpValue::Indexes(out));
        Ok(PreImageOutcome::Constant(vec![self.0.clone()]))
    }
}

pub struct SetSizeHintCommandType();

impl CommandType for SetSizeHintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","set_size_hint"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            let offset = if sig[0].get_mode() == MemberMode::FValue { 1 } else { 0 };
            Ok(Box::new(SetSizeHintCommand(hint_reg(&sig[offset],&it.regs,true)?,
                                            sig[offset].all_registers().iter().map(|x| it.regs[*x].clone()).collect(),
                                            it.regs[sig[offset+1].iter().next().unwrap().1.data_pos()])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(SetSizeHintCommand(HashSet::new(),vec![],Register(0))))
    }
}

pub struct SetSizeHintCommand(HashSet<Register>,Vec<Register>,Register);

impl Command for SetSizeHintCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }
    
    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        if !context.is_reg_valid(&self.2) {
            return Err(format!("set_size_hint needs compile-time-fixed value\n"));
        }
        let values = context.context_mut().registers_mut().get_indexes(&self.2)?;
        let mut out = vec![];
        let mut values = values.iter().cycle();
        for reg in self.1.iter() {
            let value = if self.0.contains(reg) {
                values.next().cloned()
            } else {
                context.get_reg_size(reg)
            };
            if let Some(value) = value {
                out.push((reg.clone(),value));
            }
        }
        Ok(PreImageOutcome::Skip(out))
    }
}

pub struct ForcePauseCommandType();

impl CommandType for ForcePauseCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(Identifier::new("buildtime","force_pause"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(ForcePauseCommand()))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(ForcePauseCommand()))
    }
}

pub struct ForcePauseCommand();

impl Command for ForcePauseCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }
    
    fn preimage(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Replace(vec![Instruction::new(InstructionType::Pause(true),vec![])]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { 0. }}
