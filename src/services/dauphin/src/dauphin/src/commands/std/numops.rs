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
use crate::model::{ Register, cbor_make_map };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, TimeTrialCommandType, PreImagePrepare };
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
        context.registers().write(&Register(1),InterpValue::Indexes(a));
        let b : Vec<usize> = (0..t).map(|x| (x as usize * 40503 )%t).collect();
        context.registers().write(&Register(2),InterpValue::Indexes(b));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,Register(0),Register(1),Register(2))))
    }
}

pub struct InterpBinBoolCommandType(InterpBinBoolOp);

impl CommandType for InterpBinBoolCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&BinBoolTimeTrial(self.0),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }
}

pub struct InterpBinBoolCommand(pub(crate) InterpBinBoolOp, pub(crate) Register,pub(crate) Register,pub(crate) Register);

impl Command for InterpBinBoolCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
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

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.2) && context.is_reg_valid(&self.3) {
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
}

struct BinNumTimeTrial(InterpBinNumOp);

impl TimeTrialCommandType for BinNumTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let a : Vec<usize> = (0..t).map(|x| x as usize).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(a));
        let b : Vec<usize> = (0..t).map(|x| (x as usize * 40503 )%t).collect();
        context.registers().write(&Register(2),InterpValue::Indexes(b));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(self.0,Register(0),Register(1),Register(2))))
    }
}

pub struct InterpBinNumCommandType(InterpBinNumOp);

impl CommandType for InterpBinNumCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(self.0,it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&BinNumTimeTrial(self.0),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }
}

pub struct InterpBinNumCommand(pub(crate) InterpBinNumOp, pub(crate) Register,pub(crate) Register,pub(crate) Register);

impl Command for InterpBinNumCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
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

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.2) && context.is_reg_valid(&self.3) {
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
}

struct NumModTimeTrial(InterpNumModOp,bool);

impl TimeTrialCommandType for NumModTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        let a : Vec<usize> = (1..t).map(|x| x as usize).collect();
        context.registers().write(&Register(0),InterpValue::Indexes(a));
        let b : Vec<usize> = (1..t).map(|x| (x as usize * 40503 )%t).collect();
        context.registers().write(&Register(1),InterpValue::Indexes(b));
        let f : Vec<usize> = (1..t).map(|x| x-1 as usize).collect();
        context.registers().write(&Register(2),InterpValue::Indexes(f));
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpNumModCommand(self.0,Register(0),Register(1),if self.1 { Some(Register(2)) } else { None })))
    }
}

pub struct InterpNumModCommandType(InterpNumModOp);

impl CommandType for InterpNumModCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            if sig[0].get_mode() == MemberMode::FValue {
                Ok(Box::new(InterpNumModCommand(self.0,it.regs[1].clone(),it.regs[2].clone(),Some(it.regs[0].clone()))))
            } else {
                Ok(Box::new(InterpNumModCommand(self.0,it.regs[0].clone(),it.regs[1].clone(),None)))
            }
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let filter = if *value[2] == CborValue::Null { 
            None
        } else {
            Some(Register::deserialize(value[2])?)
        };
        Ok(Box::new(InterpNumModCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            filter)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let unfiltered = TimeTrial::run(&NumModTimeTrial(self.0,false),linker,config)?;
        let filtered = TimeTrial::run(&NumModTimeTrial(self.0,true),linker,config)?;
        Ok(cbor_make_map(&vec!["tu","tf"],vec![unfiltered.serialize(),filtered.serialize()])?)
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

pub struct InterpNumModCommand(InterpNumModOp,Register,Register,Option<Register>);

impl InterpNumModCommand {
    fn execute_unfiltered(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
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
        let filter : &[usize] = &context.registers().get_indexes(self.3.as_ref().unwrap())?;
        let registers = context.registers();
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

impl Command for InterpNumModCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        if self.3.is_some() {
            self.execute_filtered(context)
        } else {
            self.execute_unfiltered(context)
        }
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
                self.3.map(|r| context.is_reg_valid(&r)).unwrap_or(true) {
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
}


pub(super) fn library_numops_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("lt",5,InterpBinBoolCommandType(InterpBinBoolOp::Lt))?;
    set.push("lteq",6,InterpBinBoolCommandType(InterpBinBoolOp::LtEq))?;
    set.push("gt",7,InterpBinBoolCommandType(InterpBinBoolOp::Gt))?;
    set.push("gteq",8,InterpBinBoolCommandType(InterpBinBoolOp::GtEq))?;
    set.push("incr",11,InterpNumModCommandType(InterpNumModOp::Incr))?;
    set.push("plus",12,InterpBinNumCommandType(InterpBinNumOp::Plus))?;
    Ok(())
}
