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
    CompilerLink, TimeTrialCommandType, TimeTrial, trial_signature
};
use dauphin_compile::model::PreImageContext;
use dauphin_interp::command::Identifier;
use dauphin_interp::types::{ MemberMode, BaseType, MemberDataFlow };
use dauphin_interp::runtime::{ InterpValue, InterpContext, Register };
use dauphin_interp::util::cbor::{ cbor_make_map, cbor_map };
use serde_cbor::Value as CborValue;
use crate::{ InterpBinBoolOp, InterpBinNumOp, InterpNumModOp };
use super::library::std;

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

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std",self.0.name()),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
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

pub struct InterpBinBoolCommand(InterpBinBoolOp,Register,Register,Register,Option<TimeTrial>);

impl Command for InterpBinBoolCommand {
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

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std",self.0.name()),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
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

pub struct InterpBinNumCommand(InterpBinNumOp,Register,Register,Register,Option<TimeTrial>);

impl Command for InterpBinNumCommand {
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

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::InOut,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std",self.0.name()),true,sig,vec![MemberDataFlow::InOut,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
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

pub struct InterpNumModCommand(InterpNumModOp,Register,Register,Option<Register>,Option<TimeTrial>);

impl Command for InterpNumModCommand {
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

pub(super) fn library_numops_commands(set: &mut CompLibRegister) -> Result<(),String> {
    set.push("lt",Some(5),InterpBinBoolCommandType::new(InterpBinBoolOp::Lt));
    set.push("lteq",Some(6),InterpBinBoolCommandType::new(InterpBinBoolOp::LtEq));
    set.push("gt",Some(7),InterpBinBoolCommandType::new(InterpBinBoolOp::Gt));
    set.push("gteq",Some(8),InterpBinBoolCommandType::new(InterpBinBoolOp::GtEq));
    set.push("incr",Some(11),InterpNumModCommandType::new(InterpNumModOp::Incr));
    set.push("plus",Some(12),InterpBinNumCommandType::new(InterpBinNumOp::Plus));
    Ok(())
}
