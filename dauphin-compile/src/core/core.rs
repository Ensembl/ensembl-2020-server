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

#[macro_use]
use super::commontype;

use crate::cli::Config;
use crate::command::{
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, PreImagePrepare,
    CompLibRegister, Instruction, InstructionType, InstructionSuperType, CompilerLink, TimeTrialCommandType, TimeTrial
};
use crate::model::PreImageContext;
use dauphin_interp::command::CommandSetId;
use dauphin_interp::runtime::{ InterpValue, InterpContext, Register };
use dauphin_interp::util::cbor::{ cbor_make_map, cbor_map };
use serde_cbor::Value as CborValue;
use super::consts::{ const_commands };
use dauphin_interp::{ make_core_interp };

struct NilTimeTrial();

impl TimeTrialCommandType for NilTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,1) }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Nil,vec![Register(0)]))
    }
}

pub struct NilCommandType(f64);

impl NilCommandType {
    fn new() -> NilCommandType { NilCommandType(1.) }
}

impl CommandType for NilCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Instruction(InstructionSuperType::Nil)
        }
    }
    
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NilCommand(it.regs[0],self.0)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&NilTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = TimeTrial::deserialize(&t[0])?.evaluate(1.);
        Ok(())
    }
}

pub struct NilCommand(Register,f64);

impl Command for NilCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        if context.is_last() {
            Ok(PreImagePrepare::Keep(vec![(self.0,1)]))
        } else {
            Ok(PreImagePrepare::Replace)
        }
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.1 }
}

struct CopyTimeTrial();

impl TimeTrialCommandType for CopyTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Copy,vec![Register(0),Register(1)]))
    }
}

type_instr2!(CopyCommandType,CopyCommand,InstructionSuperType::Copy,CopyTimeTrial);

pub struct CopyCommand(Register,Register,Option<TimeTrial>);

impl Command for CopyCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = context.get_reg_size(&self.1) {
            PreImagePrepare::Keep(vec![(self.0.clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.2.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }        
    }
}

struct AppendTimeTrial();

impl TimeTrialCommandType for AppendTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(0),InterpValue::Indexes(num.clone()));
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Append,vec![Register(0),Register(1)]))
    }
}

type_instr2!(AppendCommandType,AppendCommand,InstructionSuperType::Append,AppendTimeTrial);

pub struct AppendCommand(Register,Register,Option<TimeTrial>);

impl Command for AppendCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.0) && context.is_reg_valid(&self.1) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let (Some(a),Some(b)) = (context.get_reg_size(&self.0),context.get_reg_size(&self.1)) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a+b)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        let size = match (context.get_reg_size(&self.0),context.get_reg_size(&self.1)) {
            (Some(a),Some(b)) => Some(a+b),
            (Some(a),None) => Some(2*a),
            (None,Some(b)) => Some(2*b),
            (None,None) => None
        };
        if let Some(size) = size {
            self.2.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct LengthTimeTrial();

impl TimeTrialCommandType for LengthTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Length,vec![Register(0),Register(1)]))
    }
}

type_instr2!(LengthCommandType,LengthCommand,InstructionSuperType::Length,LengthTimeTrial);

pub struct LengthCommand(Register,Register,Option<TimeTrial>);

impl Command for LengthCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && !context.is_last() {
            PreImagePrepare::Replace
        } else {
            PreImagePrepare::Keep(vec![(self.0.clone(),1)])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(len) = context.get_reg_size(&self.1) {
            self.2.as_ref().map(|x| x.evaluate(len as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct AddTimeTrial();

impl TimeTrialCommandType for AddTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let mut num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(0),InterpValue::Indexes(num.clone()));
        for i in 0..t {
            if i*3 >= num.len() { break; }
            num[i*3] += 1;
        }
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Add,vec![Register(0),Register(1)]))
    }
}

type_instr2!(AddCommandType,AddCommand,InstructionSuperType::Add,AddTimeTrial);

pub struct AddCommand(Register,Register,Option<TimeTrial>);

impl Command for AddCommand {
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
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        let size = match (context.get_reg_size(&self.0),context.get_reg_size(&self.1)) {
            (Some(a),Some(b)) => Some(a+b),
            (Some(a),None) => Some(2*a),
            (None,Some(b)) => Some(2*b),
            (None,None) => None
        };
        if let Some(size) = size {
            self.2.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct ReFilterTimeTrial();

impl TimeTrialCommandType for ReFilterTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        let filter : Vec<usize> = (0..t/2).map(|x| (x*2) as usize).collect();
        context.registers_mut().write(&Register(2),InterpValue::Indexes(filter));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::ReFilter,vec![Register(0),Register(1),Register(2)]))
    }
}

type_instr3!(ReFilterCommandType,ReFilterCommand,InstructionSuperType::ReFilter,ReFilterTimeTrial);

pub struct ReFilterCommand(Register,Register,Register,Option<TimeTrial>);

impl Command for ReFilterCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.2) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct NumEqTimeTrial();

impl TimeTrialCommandType for NumEqTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let mut num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num.clone()));
        for i in 0..t {
            if i*3 >= num.len() { break; }
            num[i*3] += 1;
        }
        context.registers_mut().write(&Register(2),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::NumEq,vec![Register(0),Register(1),Register(2)]))
    }
}

pub struct NumEqCommandType(Option<TimeTrial>);

impl NumEqCommandType {
    fn new() -> NumEqCommandType { NumEqCommandType(None) }
}

impl CommandType for NumEqCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Instruction(InstructionSuperType::NumEq)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumEqCommand(it.regs[0],it.regs[1],it.regs[2],self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&NumEqTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct NumEqCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register,Option<TimeTrial>);

impl Command for NumEqCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.1) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }    
}

struct FilterTimeTrial();

impl TimeTrialCommandType for FilterTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        let filter : Vec<bool> = (0..t).map(|x| ((x%4)<2) as bool).collect();
        context.registers_mut().write(&Register(2),InterpValue::Boolean(filter));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Filter,vec![Register(0),Register(1),Register(2)]))
    }
}

type_instr3!(FilterCommandType,FilterCommand,InstructionSuperType::Filter,FilterTimeTrial);

pub struct FilterCommand(Register,Register,Register,Option<TimeTrial>);

impl Command for FilterCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.1) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct RunTimeTrial();

impl TimeTrialCommandType for RunTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let start : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(start));
        let len : Vec<usize> = (0..t).map(|x| (x%10) as usize).collect();
        context.registers_mut().write(&Register(2),InterpValue::Indexes(len));
        context.registers_mut().write(&Register(3),InterpValue::Indexes(vec![]));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::Run,vec![Register(0),Register(1),Register(2),Register(3)]))
    }
}

type_instr4!(RunCommandType,RunCommand,InstructionSuperType::Run,RunTimeTrial);

pub struct RunCommand(Register,Register,Register,Register,Option<TimeTrial>);

impl RunCommand {
    fn size(&self, context: &PreImageContext) -> Option<usize> {
        let mut size = match (context.get_reg_size(&self.1),context.get_reg_size(&self.2)) {
            (Some(a),Some(b)) => Some(a+b),
            (Some(a),None) => Some(2*a),
            (None,Some(b)) => Some(2*b),
            (None,None) => None
        };
        if size.is_none() {
            if let Some(a) = context.get_reg_size(&self.3) {
                size = Some(a);
            }
        }
        size
    }
}

impl Command for RunCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize(),self.3.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.2) {
            if context.is_reg_valid(&self.1) && !context.is_last() {
                PreImagePrepare::Replace
            } else {
                let lens = context.context_mut().registers_mut().get_indexes(&self.2)?;
                PreImagePrepare::Keep(vec![(self.0.clone(),lens.iter().sum())])
            }
        } else {
            if let Some(size) = self.size(context) {
                PreImagePrepare::Keep(vec![(self.0.clone(),size)])
            } else {
                PreImagePrepare::Keep(vec![])
            }
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        let size = self.size(context);
        if let Some(size) = size {
            self.4.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct AtTimeTrial();

impl TimeTrialCommandType for AtTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::At,vec![Register(0),Register(1)]))
    }
}

type_instr2!(AtCommandType,AtCommand,InstructionSuperType::At,AtTimeTrial);

pub struct AtCommand(Register,Register,Option<TimeTrial>);

impl Command for AtCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.1) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.2.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct SeqFilterTimeTrial();

impl TimeTrialCommandType for SeqFilterTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        let filter : Vec<usize> = (0..t/4).map(|x| (x*4) as usize).collect();
        context.registers_mut().write(&Register(2),InterpValue::Indexes(filter));
        let len : Vec<usize> = (0..t/4).map(|x| (x%2) as usize).collect();
        context.registers_mut().write(&Register(3),InterpValue::Indexes(len));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::SeqFilter,vec![Register(0),Register(1),Register(2),Register(3)]))
    }
}

type_instr4!(SeqFilterCommandType,SeqFilterCommand,InstructionSuperType::SeqFilter,SeqFilterTimeTrial);

pub struct SeqFilterCommand(Register,Register,Register,Register,Option<TimeTrial>);

impl Command for SeqFilterCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize(),self.3.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.3) {
            if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
                PreImagePrepare::Replace
            } else if let Some(num) = context.get_reg_size(&self.2) {
                let lens = context.context_mut().registers_mut().get_indexes(&self.3)?;
                let total : usize = (0..num).map(|i| lens[i%lens.len()]).sum();
                PreImagePrepare::Keep(vec![(self.0.clone(),total)])
            } else {
                PreImagePrepare::Keep(vec![])
            }
        } else if let Some(num) = context.get_reg_size(&self.1) {
            PreImagePrepare::Keep(vec![(self.0.clone(),num)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.4.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct SeqAtTimeTrial();

impl TimeTrialCommandType for SeqAtTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*10;
        let num : Vec<usize> = (0..t).map(|x| (x%10) as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(num));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::SeqAt,vec![Register(0),Register(1),Register(2)]))
    }
}

type_instr3!(SeqAtCommandType,SeqAtCommand,InstructionSuperType::SeqAt,SeqAtTimeTrial);

pub struct SeqAtCommand(Register,Register,Register,Option<TimeTrial>);

impl Command for SeqAtCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(x) = context.get_reg_size(&self.2) {
            PreImagePrepare::Keep(vec![(self.0.clone(),x)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/10.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

pub struct PauseCommandType();

impl CommandType for PauseCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Instruction(InstructionSuperType::Pause)
        }
    }
    fn from_instruction(&self, _it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(PauseCommand()))
    }
}

pub struct PauseCommand();

impl Command for PauseCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![]))
    }
}

pub fn make_core() -> Result<CompLibRegister,String> {
    let set_id = CommandSetId::new("core",(0,0),0x6131BA5737E6EAE0);
    let mut set = CompLibRegister::new(&set_id,Some(make_core_interp()?));
    const_commands(&mut set)?;
    set.push("nil",Some(5),NilCommandType::new());
    set.push("copy",Some(6),CopyCommandType::new());
    set.push("append",Some(7),AppendCommandType::new());
    set.push("length",Some(8),LengthCommandType::new());
    set.push("add",Some(9),AddCommandType::new());
    set.push("numeq",Some(10),NumEqCommandType::new());
    set.push("filter",Some(11),FilterCommandType::new());
    set.push("run",Some(12),RunCommandType::new());
    set.push("seqfilter",Some(13),SeqFilterCommandType::new());
    set.push("seqat",Some(14),SeqAtCommandType::new());
    set.push("at",Some(15),AtCommandType::new());
    set.push("refilter",Some(16),ReFilterCommandType::new());
    set.push("pause",Some(18),PauseCommandType());
    set.dynamic_data(include_bytes!("core-0.0.ddd"));
    Ok(set)
}
