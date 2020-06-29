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

use std::rc::Rc;
use crate::interp::InterpValue;
use crate::interp::{ Command, CommandSet, CommandSetId, InterpContext, PreImageOutcome };
use crate::model::{ Register, cbor_make_map };
use serde_cbor::Value as CborValue;
use crate::commands::common::polymorphic::arbitrate_type;
use super::consts::const_commands;
use crate::generate::{ Instruction, InstructionSuperType, PreImageContext };
use crate::interp::{ CommandSchema, CommandType, CommandTrigger, TimeTrialCommandType, TimeTrial };
use crate::cli::Config;
use crate::interp::CompilerLink;

struct NilTimeTrial();

impl TimeTrialCommandType for NilTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,1) }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NilCommand(Register(0))))
    }
}

pub struct NilCommandType();

impl CommandType for NilCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Instruction(InstructionSuperType::Nil)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NilCommand(it.regs[0])))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NilCommand(Register::deserialize(value[0])?)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&NilTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }
}

pub struct NilCommand(pub(crate) Register);

impl Command for NilCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().write(&self.0,InterpValue::Empty);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize()])
    }

    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<bool,String> { Ok(true) }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct CopyTimeTrial();

impl TimeTrialCommandType for CopyTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(CopyCommand(Register(0),Register(1))))
    }
}

type_instr2!(CopyCommandType,CopyCommand,InstructionSuperType::Copy,CopyTimeTrial);

pub struct CopyCommand(pub(crate) Register,pub(crate) Register);

impl Command for CopyCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers().copy(&self.0,&self.1)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct AppendTimeTrial();

impl TimeTrialCommandType for AppendTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(0),InterpValue::Indexes(num.clone()));
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AppendCommand(Register(0),Register(1))))
    }
}

type_instr2!(AppendCommandType,AppendCommand,InstructionSuperType::Append,AppendTimeTrial);

pub struct AppendCommand(pub(crate) Register,pub(crate) Register);

fn append_typed<T>(dst: &mut Vec<T>, src: &Vec<T>) where T: Clone {
    dst.append(&mut src.clone());
}

fn append(dst: InterpValue, src: &Rc<InterpValue>) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&dst,src,false) {
        Ok(polymorphic!(dst,[src],natural,(|d,s| {
            append_typed(d,s)
        })))
    } else {
        Ok(dst)
    }
}

impl Command for AppendCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1).borrow().get_shared()?;
        let dstr = registers.get(&self.0);
        let dst = dstr.borrow_mut().get_exclusive()?;
        registers.write(&self.0,append(dst,&src)?);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.0) && context.get_reg_valid(&self.1))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct LengthTimeTrial();

impl TimeTrialCommandType for LengthTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(LengthCommand(Register(0),Register(1))))
    }
}

type_instr2!(LengthCommandType,LengthCommand,InstructionSuperType::Length,LengthTimeTrial);

pub struct LengthCommand(pub(crate) Register,pub(crate) Register);

impl Command for LengthCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let len = registers.get(&self.1).borrow().get_shared()?.len();
        registers.write(&self.0,InterpValue::Indexes(vec![len]));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct AddTimeTrial();

impl TimeTrialCommandType for AddTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let mut num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(0),InterpValue::Indexes(num.clone()));
        for i in 0..t {
            if i*3 >= num.len() { break; }
            num[i*3] += 1;
        }
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AddCommand(Register(0),Register(1))))
    }
}

type_instr2!(AddCommandType,AddCommand,InstructionSuperType::Add,AddTimeTrial);

pub struct AddCommand(pub(crate) Register,pub(crate) Register);

impl Command for AddCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = registers.take_indexes(&self.0)?;
        let src_len = (&src).len();
        for i in 0..dst.len() {
            dst[i] += src[i%src_len];
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.0) && context.get_reg_valid(&self.1))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct ReFilterTimeTrial();

impl TimeTrialCommandType for ReFilterTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        let filter : Vec<usize> = (0..t/2).map(|x| (x*2) as usize).collect();
        context.registers().write(&Register(2),InterpValue::Indexes(filter));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(ReFilterCommand(Register(0),Register(1),Register(2))))
    }
}

type_instr3!(ReFilterCommandType,ReFilterCommand,InstructionSuperType::ReFilter,ReFilterTimeTrial);

pub struct ReFilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for ReFilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src : &[usize] = &registers.get_indexes(&self.1)?;
        let indexes : &[usize] = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        for x in indexes.iter() {
            dst.push(src[*x]);
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct NumEqTimeTrial();

impl TimeTrialCommandType for NumEqTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let mut num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num.clone()));
        for i in 0..t {
            if i*3 >= num.len() { break; }
            num[i*3] += 1;
        }
        context.registers().write(&Register(2),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumEqCommand(Register(0),Register(1),Register(2))))
    }
}

pub struct NumEqCommandType();

impl CommandType for NumEqCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Instruction(InstructionSuperType::NumEq)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumEqCommand(it.regs[0],it.regs[1],it.regs[2])))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumEqCommand(Register::deserialize(value[0])?,Register::deserialize(value[1])?,Register::deserialize(value[2])?)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&NumEqTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }
}

pub struct NumEqCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for NumEqCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src1 = &registers.get_indexes(&self.1)?;
        let src2 = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        let src2len = src2.len();
        for i in 0..src1.len() {
            dst.push(src1[i] == src2[i%src2len]);
        }
        registers.write(&self.0,InterpValue::Boolean(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

fn filter_typed<T>(dst: &mut Vec<T>, src: &[T], filter: &[bool]) where T: Clone {
    let filter_len = filter.len();
    for (i,value) in src.iter().enumerate() {
        if filter[i%filter_len] {
            dst.push(value.clone());
        }
    }
}

pub fn filter(src: &Rc<InterpValue>, filter_val: &[bool]) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&InterpValue::Empty,src,true) {
        Ok(polymorphic!(InterpValue::Empty,[src],natural,(|d,s| {
            filter_typed(d,s,filter_val)
        })))
    } else {
        Ok(InterpValue::Empty)
    }
}

struct FilterTimeTrial();

impl TimeTrialCommandType for FilterTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        let filter : Vec<bool> = (0..t).map(|x| ((x%4)<2) as bool).collect();
        context.registers().write(&Register(2),InterpValue::Boolean(filter));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(FilterCommand(Register(0),Register(1),Register(2))))
    }
}

type_instr3!(FilterCommandType,FilterCommand,InstructionSuperType::Filter,FilterTimeTrial);

pub struct FilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for FilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let filter_val = registers.get_boolean(&self.2)?;
        let src = registers.get(&self.1);
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,filter(&src,&filter_val)?);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct RunTimeTrial();

impl TimeTrialCommandType for RunTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let start : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(start));
        let len : Vec<usize> = (0..t).map(|x| (x%10) as usize).collect();
        context.registers().write(&Register(2),InterpValue::Indexes(len));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(RunCommand(Register(0),Register(1),Register(2))))
    }
}

type_instr3!(RunCommandType,RunCommand,InstructionSuperType::Run,RunTimeTrial);

pub struct RunCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register);

impl Command for RunCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let start = &registers.get_indexes(&self.1)?;
        let len = &registers.get_indexes(&self.2)?;
        let mut dst = vec![];
        let startlen = start.len();
        let lenlen = len.len();
        for i in 0..startlen {
            for j in 0..len[i%lenlen] {
                dst.push(start[i]+j);
            }
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct AtTimeTrial();

impl TimeTrialCommandType for AtTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AtCommand(Register(0),Register(1))))
    }
}

type_instr2!(AtCommandType,AtCommand,InstructionSuperType::At,AtTimeTrial);

pub struct AtCommand(pub(crate) Register, pub(crate) Register);

impl Command for AtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = vec![];
        for i in 0..src.len() {
            dst.push(i);
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

fn seq_filter_typed<T>(dst: &mut Vec<T>, src: &[T], starts: &[usize], lens: &[usize]) where T: Clone {
    let starts_len = starts.len();
    let lens_len = lens.len();
    let src_len = src.len();
    for i in 0..starts_len {
        for j in 0..lens[i%lens_len] {
            dst.push(src[(starts[i]+j)%src_len].clone());
        }
    }
}

fn seq_filter(src: &Rc<InterpValue>, starts: &[usize], lens: &[usize]) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&InterpValue::Empty,src,true) {
        Ok(polymorphic!(InterpValue::Empty,[src],natural,(|d,s| {
            seq_filter_typed(d,s,starts,lens)
        })))
    } else {
        Ok(InterpValue::Empty)
    }
}

struct SeqFilterTimeTrial();

impl TimeTrialCommandType for SeqFilterTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        let filter : Vec<usize> = (0..t/4).map(|x| (x*4) as usize).collect();
        context.registers().write(&Register(2),InterpValue::Indexes(filter));
        let len : Vec<usize> = (0..t/4).map(|x| (x%2) as usize).collect();
        context.registers().write(&Register(3),InterpValue::Indexes(len));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(SeqFilterCommand(Register(0),Register(1),Register(2),Register(3))))
    }
}

type_instr4!(SeqFilterCommandType,SeqFilterCommand,InstructionSuperType::SeqFilter,SeqFilterTimeTrial);

pub struct SeqFilterCommand(pub(crate) Register,pub(crate) Register, pub(crate) Register, pub(crate) Register);

impl Command for SeqFilterCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = registers.get(&self.1);
        let start = registers.get_indexes(&self.2)?;
        let len = registers.get_indexes(&self.3)?;
        let src = src.borrow().get_shared()?;
        registers.write(&self.0,seq_filter(&src,&start,&len)?);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize(),self.2.serialize(),self.3.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1) && context.get_reg_valid(&self.2) && context.get_reg_valid(&self.3))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }
}

struct SeqAtTimeTrial();

impl TimeTrialCommandType for SeqAtTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t*10;
        let num : Vec<usize> = (0..t).map(|x| (x%10) as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(num));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(SeqAtCommand(Register(0),Register(1))))
    }
}

type_instr2!(SeqAtCommandType,SeqAtCommand,InstructionSuperType::SeqAt,SeqAtTimeTrial);

pub struct SeqAtCommand(pub(crate) Register,pub(crate) Register);

impl Command for SeqAtCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let src = &registers.get_indexes(&self.1)?;
        let mut dst = vec![];
        for i in 0..src.len() {
            for j in 0..src[i] {
                dst.push(j);
            }
        }
        registers.write(&self.0,InterpValue::Indexes(dst));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<bool,String> { 
        Ok(context.get_reg_valid(&self.1))
    }
    
    fn preimage_post(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        context.set_reg_valid(&self.0,true);
        Ok(PreImageOutcome::Constant(vec![self.0]))
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

    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(PauseCommand()))
    }
}

pub struct PauseCommand();

impl Command for PauseCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.do_pause();
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![])
    }
}

pub fn make_core() -> Result<CommandSet,String> {
    let set_id = CommandSetId::new("core",(0,0),0xD8DA0F075C671A8A);
    let mut set = CommandSet::new(&set_id,false);
    const_commands(&mut set)?;
    set.push("nil",5,NilCommandType())?;
    set.push("copy",6,CopyCommandType())?;
    set.push("append",7,AppendCommandType())?;
    set.push("length",8,LengthCommandType())?;
    set.push("add",9,AddCommandType())?;
    set.push("numeq",10,NumEqCommandType())?;
    set.push("filter",11,FilterCommandType())?;
    set.push("run",12,RunCommandType())?;
    set.push("seqfilter",13,SeqFilterCommandType())?;
    set.push("seqat",14,SeqAtCommandType())?;
    set.push("at",15,AtCommandType())?;
    set.push("refilter",16,ReFilterCommandType())?;
    set.push("pause",18,PauseCommandType())?;
    set.load_dynamic_data(include_bytes!("core-0.0.ddd"))?;
    Ok(set)
}
