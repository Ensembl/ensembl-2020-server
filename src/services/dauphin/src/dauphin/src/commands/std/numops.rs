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

use crate::interp::{ InterpValue, PreImageOutcome, TimeTrial };
use crate::model::{ Register, cbor_make_map, cbor_map };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, TimeTrialCommandType, PreImagePrepare, InterpCommand };
use crate::generate::{ Instruction, PreImageContext, InstructionType };
use serde_cbor::Value as CborValue;
use super::library::std;
use crate::cli::Config;
use crate::interp::CompilerLink;
use crate::typeinf::MemberMode;

#[derive(Copy,Clone)]
pub enum InterpBinNumOp {
    Plus
}

impl InterpBinNumOp {
    fn evaluate(&self, a: f64, b: f64) -> f64 {
        match self {
            InterpBinNumOp::Plus => a + b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpBinNumOp::Plus => "plus",
        }
    }
}

#[derive(Copy,Clone)]
pub enum InterpBinBoolOp {
    Lt,
    LtEq,
    Gt,
    GtEq
}

impl InterpBinBoolOp {
    fn evaluate(&self, a: f64, b: f64) -> bool {
        match self {
            InterpBinBoolOp::Lt => a < b,
            InterpBinBoolOp::LtEq => a <= b,
            InterpBinBoolOp::Gt => a > b,
            InterpBinBoolOp::GtEq => a >= b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpBinBoolOp::Lt => "lt",
            InterpBinBoolOp::LtEq => "lteq",
            InterpBinBoolOp::Gt => "gt",
            InterpBinBoolOp::GtEq => "gteq"
        }
    }
}

struct BinBoolTimeTrial(InterpBinBoolOp);

impl TimeTrialCommandType for BinBoolTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let a : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(a));
        let b : Vec<usize> = (0..t).map(|x| (x as usize * 40503 )%t).collect();
        context.registers_mut().write(&Register(2),InterpValue::Indexes(b));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,Register(0),Register(1),Register(2),None)))
    }
}

pub struct InterpBinBoolCommandType(InterpBinBoolOp,Option<TimeTrial>);

impl InterpBinBoolCommandType {
    fn new(op: InterpBinBoolOp) -> InterpBinBoolCommandType { InterpBinBoolCommandType(op,None) }
}

impl CommandType for InterpBinBoolCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,it.regs[0],it.regs[1],it.regs[2],self.1.clone())))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?,
            self.1.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&BinBoolTimeTrial(self.0),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.1 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct InterpBinBoolInterpCommand(InterpBinBoolOp,Register,Register,Register);

impl InterpCommand for InterpBinBoolInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let a = registers.get_numbers(&self.2)?;
        let b = &registers.get_numbers(&self.3)?;
        let mut c = vec![];
        let b_len = b.len();
        for (i,a_val) in a.iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        registers.write(&self.1,InterpValue::Boolean(c));
        Ok(())
    }
}

pub struct InterpBinBoolCommand(InterpBinBoolOp,Register,Register,Register,Option<TimeTrial>);

impl Command for InterpBinBoolCommand {
    fn to_interp_command(&self) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(InterpBinBoolInterpCommand(self.0,self.1,self.2,self.3)))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.2) && context.is_reg_valid(&self.3) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.2) {
            PreImagePrepare::Keep(vec![(self.1.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.1]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.2) {
            self.4.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct BinNumTimeTrial(InterpBinNumOp);

impl TimeTrialCommandType for BinNumTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let a : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(a));
        let b : Vec<usize> = (0..t).map(|x| (x as usize * 40503 )%t).collect();
        context.registers_mut().write(&Register(2),InterpValue::Indexes(b));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(self.0,Register(0),Register(1),Register(2),None)))
    }
}

pub struct InterpBinNumCommandType(InterpBinNumOp,Option<TimeTrial>);

impl InterpBinNumCommandType {
    fn new(op: InterpBinNumOp) -> InterpBinNumCommandType { InterpBinNumCommandType(op,None) }
}

impl CommandType for InterpBinNumCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(self.0,it.regs[0],it.regs[1],it.regs[2],None)))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?,
            self.1.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&BinNumTimeTrial(self.0),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.1 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct InterpBinNumInterpCommand(InterpBinNumOp,Register,Register,Register);

impl InterpCommand for InterpBinNumInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let a = registers.get_numbers(&self.2)?;
        let b = &registers.get_numbers(&self.3)?;
        let mut c = vec![];
        let b_len = b.len();
        for (i,a_val) in a.iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        registers.write(&self.1,InterpValue::Numbers(c));
        Ok(())
    }
}

pub struct InterpBinNumCommand(InterpBinNumOp,Register,Register,Register,Option<TimeTrial>);

impl Command for InterpBinNumCommand {
    fn to_interp_command(&self) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(InterpBinNumInterpCommand(self.0,self.1,self.2,self.3)))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.2) && context.is_reg_valid(&self.3) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.2) {
            PreImagePrepare::Keep(vec![(self.1.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.1]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.2) {
            self.4.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct NumModTimeTrial(InterpNumModOp,bool);

impl TimeTrialCommandType for NumModTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let a : Vec<usize> = (1..t).map(|x| x as usize).collect();
        context.registers_mut().write(&Register(0),InterpValue::Indexes(a));
        let b : Vec<usize> = (1..t).map(|x| (x as usize * 40503 )%t).collect();
        context.registers_mut().write(&Register(1),InterpValue::Indexes(b));
        let f : Vec<usize> = (1..t).map(|x| x-1 as usize).collect();
        context.registers_mut().write(&Register(2),InterpValue::Indexes(f));
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpNumModCommand(self.0,Register(0),Register(1),if self.1 { Some(Register(2)) } else { None },None)))
    }
}

pub struct InterpNumModCommandType(InterpNumModOp,Option<TimeTrial>,Option<TimeTrial>);

impl InterpNumModCommandType {
    fn new(op: InterpNumModOp) -> InterpNumModCommandType { InterpNumModCommandType(op,None,None) }
}

impl CommandType for InterpNumModCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            if sig[0].get_mode() == MemberMode::Filter {
                Ok(Box::new(InterpNumModCommand(self.0,it.regs[1].clone(),it.regs[2].clone(),Some(it.regs[0].clone()),None)))
            } else {
                Ok(Box::new(InterpNumModCommand(self.0,it.regs[0].clone(),it.regs[1].clone(),None,None)))
            }
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let (filter,trial) = if *value[2] == CborValue::Null { 
            (None,self.2.clone())
        } else {
            (Some(Register::deserialize(value[2])?),self.1.clone())
        };
        Ok(Box::new(InterpNumModCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            filter,trial)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let unfiltered = TimeTrial::run(&NumModTimeTrial(self.0,false),linker,config)?;
        let filtered = TimeTrial::run(&NumModTimeTrial(self.0,true),linker,config)?;
        Ok(cbor_make_map(&vec!["tu","tf"],vec![unfiltered.serialize(),filtered.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["tu","tf"])?;
        self.1 = Some(TimeTrial::deserialize(&t[0])?);
        self.2 = Some(TimeTrial::deserialize(&t[1])?);
        Ok(())
    }
}

#[derive(Copy,Clone)]
pub enum InterpNumModOp {
    Incr
}

impl InterpNumModOp {
    fn evaluate(&self, a: &mut f64, b: f64) {
        match self {
            InterpNumModOp::Incr => *a += b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpNumModOp::Incr => "incr",
        }
    }
}

pub struct InterpNumModInterpCommand(InterpNumModOp,Register,Register,Option<Register>);

impl InterpNumModInterpCommand {
    fn execute_unfiltered(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let b = &registers.get_numbers(&self.2)?;
        let mut a = registers.take_numbers(&self.1)?;
        let b_len = b.len();
        for (i,a_val) in a.iter_mut().enumerate() {
            self.0.evaluate(a_val,b[i%b_len]);
        }
        registers.write(&self.1,InterpValue::Numbers(a));
        Ok(())
    }

    fn execute_filtered(&self, context: &mut InterpContext) -> Result<(),String> {
        let filter : &[usize] = &context.registers_mut().get_indexes(self.3.as_ref().unwrap())?;
        let registers = context.registers_mut();
        let b = &registers.get_numbers(&self.2)?;
        let mut a = registers.take_numbers(&self.1)?;
        let b_len = b.len();
        for (i,pos) in filter.iter().enumerate() {
            self.0.evaluate(&mut a[*pos],b[i%b_len]);
        }
        registers.write(&self.1,InterpValue::Numbers(a));
        Ok(())
    }
}

impl InterpCommand for InterpNumModInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        if self.3.is_some() {
            self.execute_filtered(context)
        } else {
            self.execute_unfiltered(context)
        }
    }
}

pub struct InterpNumModCommand(InterpNumModOp,Register,Register,Option<Register>,Option<TimeTrial>);

impl Command for InterpNumModCommand {
    fn to_interp_command(&self) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(InterpNumModInterpCommand(self.0,self.1,self.2,self.3)))
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        if let Some(filter) = self.3 {
            Ok(Some(vec![self.1.serialize(),self.2.serialize(),filter.serialize()]))
        } else {
            Ok(Some(vec![self.1.serialize(),self.2.serialize(),CborValue::Null]))
        }
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && 
                self.3.map(|r| context.is_reg_valid(&r)).unwrap_or(true) && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.1) {
            PreImagePrepare::Keep(vec![(self.1.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.1]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.1) {
            self.4.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}


pub(super) fn library_numops_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("lt",5,InterpBinBoolCommandType::new(InterpBinBoolOp::Lt))?;
    set.push("lteq",6,InterpBinBoolCommandType::new(InterpBinBoolOp::LtEq))?;
    set.push("gt",7,InterpBinBoolCommandType::new(InterpBinBoolOp::Gt))?;
    set.push("gteq",8,InterpBinBoolCommandType::new(InterpBinBoolOp::GtEq))?;
    set.push("incr",11,InterpNumModCommandType::new(InterpNumModOp::Incr))?;
    set.push("plus",12,InterpBinNumCommandType::new(InterpBinNumOp::Plus))?;
    Ok(())
}
