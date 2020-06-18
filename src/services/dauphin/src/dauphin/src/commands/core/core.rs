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
use std::time::{ SystemTime, Duration };
use crate::interp::InterpValue;
use crate::interp::{ Command, CommandSet, CommandSetId, InterpContext, PreImageOutcome };
use crate::model::{ Register, cbor_make_map };
use serde_cbor::Value as CborValue;
use super::commontype::BuiltinCommandType;
use crate::commands::common::polymorphic::arbitrate_type;
use super::consts::const_commands;
use crate::generate::{ Instruction, InstructionSuperType, PreImageContext };
use crate::interp::{ CommandSchema, CommandType, CommandTrigger };
use crate::cli::Config;
use crate::interp::CompilerLink;

fn regress(input: &[(u64,f64)]) -> Result<f64,String> {
    if input.len() == 0 {
        return Err("no data to regress".to_string());
    }
    let total_x : u64 = input.iter().map(|x| x.0).sum();
    let total_y : f64 = input.iter().map(|x| x.1).sum();
    let mean_x = total_x as f64 / input.len() as f64;
    let mean_y = total_y / input.len() as f64;
    let mut numer = 0.;
    let mut denom = 0.;
    for (x,y) in input {
        let x_delta = *x as f64 - mean_x;
        let y_delta = y         - mean_y;
        numer += x_delta*y_delta;
        denom += x_delta*x_delta;
    }
    if denom == 0. {
        return Err("no x-variance to regress".to_string());
    }
    Ok(numer/denom)
}

pub struct NilCommandType();

impl NilCommandType {
    fn run_time_trial(&self, linker: &CompilerLink, _config: &Config, loops: u64) -> Result<f64,String> {
        let command = NilCommand(Register(0));
        let mut context = linker.new_context();
        let start_time = SystemTime::now();
        for _ in 0..loops {
            command.execute(&mut context)?;
            context.registers().commit();
        }
        Ok(start_time.elapsed().unwrap_or(Duration::new(0,0)).as_secs_f64()*1000.)
    }

    fn generate_timings(&self, linker: &CompilerLink, config: &Config) -> Result<f64,String> {
        let mut data = vec![];
        for i in 0..101 {
            let t = self.run_time_trial(linker,config,i*100)?;
            data.push((i*100,t));
            if config.get_verbose() > 2 {
                print!("loops={} time={:.2}ms\n",i*100,t);
            }
        }
        let r = regress(&data)?;
        if config.get_verbose() > 1 {
            print!("nil takes {:.3}ms\n",r);
        }
        Ok(r)
    }
}

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
        Ok(cbor_make_map(&vec!["t"],vec![CborValue::Float(self.generate_timings(linker,config)?)])?)
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
    set.push("copy",6,BuiltinCommandType::new(InstructionSuperType::Copy,2,Box::new(|x| Ok(Box::new(CopyCommand(x[0],x[1]))))))?;
    set.push("append",7,BuiltinCommandType::new(InstructionSuperType::Append,2,Box::new(|x| Ok(Box::new(AppendCommand(x[0],x[1]))))))?;
    set.push("length",8,BuiltinCommandType::new(InstructionSuperType::Length,2,Box::new(|x| Ok(Box::new(LengthCommand(x[0],x[1]))))))?;
    set.push("add",9,BuiltinCommandType::new(InstructionSuperType::Add,2,Box::new(|x| Ok(Box::new(AddCommand(x[0],x[1]))))))?;
    set.push("numeq",10,BuiltinCommandType::new(InstructionSuperType::NumEq,3,Box::new(|x| Ok(Box::new(NumEqCommand(x[0],x[1],x[2]))))))?;
    set.push("filter",11,BuiltinCommandType::new(InstructionSuperType::Filter,3,Box::new(|x| Ok(Box::new(FilterCommand(x[0],x[1],x[2]))))))?;
    set.push("run",12,BuiltinCommandType::new(InstructionSuperType::Run,3,Box::new(|x| Ok(Box::new(RunCommand(x[0],x[1],x[2]))))))?;
    set.push("seqfilter",13,BuiltinCommandType::new(InstructionSuperType::SeqFilter,4,Box::new(|x| Ok(Box::new(SeqFilterCommand(x[0],x[1],x[2],x[3]))))))?;
    set.push("seqat",14,BuiltinCommandType::new(InstructionSuperType::SeqAt,2,Box::new(|x| Ok(Box::new(SeqAtCommand(x[0],x[1]))))))?;
    set.push("at",15,BuiltinCommandType::new(InstructionSuperType::At,2,Box::new(|x| Ok(Box::new(AtCommand(x[0],x[1]))))))?;
    set.push("refilter",16,BuiltinCommandType::new(InstructionSuperType::ReFilter,3,Box::new(|x| Ok(Box::new(ReFilterCommand(x[0],x[1],x[2]))))))?;
    set.push("pause",18,BuiltinCommandType::new(InstructionSuperType::Pause,0,Box::new(|_| Ok(Box::new(PauseCommand())))))?;
    Ok(set)
}
