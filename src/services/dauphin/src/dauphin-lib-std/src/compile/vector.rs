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

use dauphin_compile::cli::Config;
use dauphin_compile::command::{
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, PreImagePrepare, CompLibRegister, Instruction, InstructionType, 
    CompilerLink, TimeTrialCommandType, TimeTrial, trial_signature, trial_write
};
use dauphin_compile::model::PreImageContext;
use dauphin_interp::command::Identifier;
use dauphin_interp::types::{ MemberMode, BaseType, MemberDataFlow };
use dauphin_interp::runtime::{ InterpContext, Register };
use dauphin_interp::util::cbor::{ cbor_make_map, cbor_map };
use serde_cbor::Value as CborValue;
use super::library::std;

struct VectorCopyShallowTimeTrial();

impl TimeTrialCommandType for VectorCopyShallowTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*100,|x| x);
        trial_write(context,1,t*100,|x| x);
        trial_write(context,2,t*100,|x| x);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_vector_copy_shallow"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
    }
}

pub struct VectorCopyShallowType(Option<TimeTrial>);

impl VectorCopyShallowType {
    fn new() -> VectorCopyShallowType { VectorCopyShallowType(None) }
}

impl CommandType for VectorCopyShallowType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std("_vector_copy_shallow"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorCopyShallow(it.regs[0].clone(),it.regs[1].clone(),it.regs[2].clone(),self.0.clone())))
    }
    
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&VectorCopyShallowTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct VectorCopyShallow(Register,Register,Register,Option<TimeTrial>);

impl VectorCopyShallow {
    fn size(&self, context: &PreImageContext) -> Option<usize> {
        let unit = if let Some(size) = context.get_reg_size(&self.1) {
            size
        } else {
            return None
        };
        let copies = if let Some(size) = context.get_reg_size(&self.2) {
            size
        } else {
            return None
        };
        Some(unit*copies)
    }
}

impl Command for VectorCopyShallow {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = self.size(context) {
            PreImagePrepare::Keep(vec![(self.0.clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = self.size(context) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct VectorAppendTimeTrial();

impl TimeTrialCommandType for VectorAppendTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*100,|x| x);
        trial_write(context,1,t*100,|x| x);
        trial_write(context,2,1,|_| 1);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_vector_append"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
    }
}

pub struct VectorAppendType(Option<TimeTrial>);

impl VectorAppendType {
    fn new() -> VectorAppendType { VectorAppendType(None) }
}

impl CommandType for VectorAppendType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std("_vector_append"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorAppend(it.regs[0].clone(),it.regs[1].clone(),it.regs[2].clone(),self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&VectorAppendTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct VectorAppend(Register,Register,Register,Option<TimeTrial>);

impl VectorAppend {
    fn size(&self, context: &PreImageContext) -> Result<Option<usize>,String> {
        let orig = if let Some(size) = context.get_reg_size(&self.0) {
            size
        } else {
            return Ok(None)
        };
        let unit = if let Some(size) = context.get_reg_size(&self.1) {
            size
        } else {
            return Ok(None)
        };
        let copies = if let Some(size) = context.get_reg_size(&self.2) {
            size
        } else {
            return Ok(None)
        };
        Ok(Some(orig+unit*copies))
    }
}

impl Command for VectorAppend {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.0) && context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = self.size(context)? {
            PreImagePrepare::Keep(vec![(self.0.clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = self.size(context).unwrap_or(None) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct VectorAppendIndexesTimeTrial();

impl TimeTrialCommandType for VectorAppendIndexesTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*100,|x| x);
        trial_write(context,1,t*10,|x| x);
        trial_write(context,2,1,|_| 0);
        trial_write(context,3,1,|_| 0);
        trial_write(context,4,10,|_| 1);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),
                                        (MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_vector_append_indexes"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2),Register(3),Register(4)]))
    }
}

pub struct VectorAppendIndexesType(Option<TimeTrial>);

impl VectorAppendIndexesType {
    fn new() -> VectorAppendIndexesType { VectorAppendIndexesType(None) }
}

impl CommandType for VectorAppendIndexesType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 5,
            trigger: CommandTrigger::Command(std("_vector_append_indexes"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorAppendIndexes(it.regs[0].clone(),it.regs[1].clone(),it.regs[2].clone(),
                                        it.regs[3].clone(),it.regs[4].clone(),
                                        self.0.clone())))
    }
    
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&VectorAppendIndexesTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct VectorAppendIndexes(Register,Register,Register,Register,Register,Option<TimeTrial>);

impl VectorAppendIndexes {
    fn size(&self, context: &PreImageContext) -> Result<Option<usize>,String> {
        let orig = if let Some(size) = context.get_reg_size(&self.0) {
            size
        } else {
            return Ok(None)
        };
        let unit = if let Some(size) = context.get_reg_size(&self.1) {
            size
        } else {
            return Ok(None)
        };
        let copies = if let Some(size) = context.get_reg_size(&self.4) {
            size
        } else {
            return Ok(None)
        };
        Ok(Some(orig+unit*copies))
    }
}

impl Command for VectorAppendIndexes {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize(),self.3.serialize(),self.4.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.0) && context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) &&
                context.is_reg_valid(&self.3) && context.is_reg_valid(&self.4) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = self.size(context)? {
            PreImagePrepare::Keep(vec![(self.0.clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = self.size(context).unwrap_or(None) {
            self.5.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct VectorUpdateIndexesTimeTrial();

impl TimeTrialCommandType for VectorUpdateIndexesTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*100,|x| x);
        trial_write(context,1,t*100,|x| x);
        trial_write(context,2,t*100,|x| x);
        trial_write(context,3,1,|_| 0);
        trial_write(context,4,1,|_| 1);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),
                                        (MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_vector_update_indexes"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2),Register(3),Register(4)]))
    }
}

pub struct VectorUpdateIndexesType(Option<TimeTrial>);

impl VectorUpdateIndexesType {
    fn new() -> VectorUpdateIndexesType { VectorUpdateIndexesType(None) }
}

impl CommandType for VectorUpdateIndexesType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 5,
            trigger: CommandTrigger::Command(std("_vector_update_indexes"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorUpdateIndexes(it.regs[0].clone(),it.regs[1].clone(),it.regs[2].clone(),
                                        it.regs[3].clone(),it.regs[4].clone(),self.0.clone())))
    }
    
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&VectorUpdateIndexesTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct VectorUpdateIndexes(Register,Register,Register,Register,Register,Option<TimeTrial>);

impl VectorUpdateIndexes {
    fn size(&self, context: &PreImageContext) -> Result<Option<usize>,String> {
        let orig = if let Some(size) = context.get_reg_size(&self.0) {
            size
        } else {
            return Ok(None)
        };
        let unit = if let Some(size) = context.get_reg_size(&self.1) {
            size
        } else {
            return Ok(None)
        };
        let copies = if context.is_reg_valid(&self.4) {
            let copies = context.context().registers().get_indexes(&self.4)?;
            copies[0]
        } else {
            return Ok(None)
        };
        Ok(Some(orig+unit*copies))
    }
}

impl Command for VectorUpdateIndexes {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize(),self.3.serialize(),self.4.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.0) && context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) &&
                context.is_reg_valid(&self.3) && context.is_reg_valid(&self.4) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = self.size(context)? {
            PreImagePrepare::Keep(vec![(self.0.clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = self.size(context).unwrap_or(None) {
            self.5.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

pub(super) fn library_vector_commands(set: &mut CompLibRegister) -> Result<(),String> {
    set.push("_vector_copy_shallow",Some(9),VectorCopyShallowType::new());
    set.push("_vector_append",Some(10),VectorAppendType::new());
    set.push("_vector_append_indexes",Some(17),VectorAppendIndexesType::new());
    set.push("_vector_update_indexes",Some(18),VectorUpdateIndexesType::new());
    Ok(())
}
