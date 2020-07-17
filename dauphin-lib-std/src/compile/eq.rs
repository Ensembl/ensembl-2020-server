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
use dauphin_interp::command::{ Identifier, InterpCommand };
use dauphin_interp::types::{
    VectorRegisters, MemberMode, RegisterSignature,
    BaseType, MemberDataFlow, FullType, ComplexPath
};
use dauphin_interp::runtime::{ InterpContext, Register };
use dauphin_interp::util::cbor::{ cbor_make_map, cbor_map };
use serde_cbor::Value as CborValue;
use super::library::std;

struct EqCompareTimeTrial();

impl TimeTrialCommandType for EqCompareTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        /* 3-deep vec (x2) */
        for i in 0..2 {
            trial_write(context,i*7+1,t*100,|x| x);
            trial_write(context,i*7+2,t*100,|x| x);
            trial_write(context,i*7+3,t*100,|_| 1);
            trial_write(context,i*7+4,t*100,|x| x);
            trial_write(context,i*7+5,t*100,|_| 1);
            trial_write(context,i*7+6,t*100,|x| x);
            trial_write(context,i*7+7,t*100,|_| 1);
        }
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let regs = (0..15).map(|i| Register(i)).collect();
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,3,BaseType::NumberType),(MemberMode::In,3,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_eq_compare"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),regs))
    }
}

pub struct EqCompareCommandType(Option<TimeTrial>);

impl EqCompareCommandType {
    fn new() -> EqCompareCommandType { EqCompareCommandType(None) }
}

impl CommandType for EqCompareCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std("_eq_compare"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            let a = sig[1].iter().next().ok_or_else(|| format!("bad conversion"))?;
            let b = sig[2].iter().next().ok_or_else(|| format!("bad conversion"))?;
            Ok(Box::new(EqCompareCommand(a.1.clone(),b.1.clone(),it.regs.to_vec(),self.0.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&EqCompareTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct EqCompareCommand(VectorRegisters,VectorRegisters,Vec<Register>,Option<TimeTrial>);

impl EqCompareCommand {
    fn enough_valid(&self, context: &PreImageContext) -> Result<bool,String> {
        for i in 1..self.2.len() {
            if !context.is_reg_valid(&self.2[i]) { return Ok(false); }
        }
        Ok(true)
    }

    fn any_size(&self, context: &PreImageContext) -> Option<usize> {
        for i in 1..self.2.len() {
            if let Some(size) = context.get_reg_size(&self.2[i]) {
                return Some(size);
            }
        }
        None
    }
}

impl Command for EqCompareCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.2.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![self.0.serialize(true)?,self.1.serialize(true)?,regs]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if self.enough_valid(context)? && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = self.any_size(context) {
            PreImagePrepare::Keep(vec![(self.2[0].clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.2[0].clone()]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if self.0.depth() > 3 { return 1.; }
        if let Some(size) = context.get_reg_size(&self.2[1]) { /* [1] ie data of first vector */
            self.3.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct EqShallowTimeTrial();

impl TimeTrialCommandType for EqShallowTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,1,t*100,|x| x);
        trial_write(context,2,t*100,|x| x);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_eq_shallow"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
    }
}

pub struct EqShallowCommandType(Option<TimeTrial>);

impl EqShallowCommandType {
    fn new () -> EqShallowCommandType { EqShallowCommandType(None) }
}

impl CommandType for EqShallowCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std("_eq_shallow"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(EqShallowCommand(it.regs[0].clone(),it.regs[1].clone(),it.regs[2].clone(),self.0.clone())))
    }
    
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&EqShallowTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct EqShallowCommand(Register,Register,Register,Option<TimeTrial>);

impl Command for EqShallowCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) && !context.is_last() {
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
            self.3.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

struct EqAllTimeTrial();

impl TimeTrialCommandType for EqAllTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,1,t*100,|_| 1);
        trial_write(context,2,t*100,|_| 0);
        context.registers_mut().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let sig = trial_signature(&vec![(MemberMode::Out,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType),(MemberMode::In,0,BaseType::NumberType)]);
        Ok(Instruction::new(InstructionType::Call(Identifier::new("std","_eq_all"),true,sig,vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),
            vec![Register(0),Register(1),Register(2)]))
    }
}

pub struct AllCommandType(Option<TimeTrial>);

impl AllCommandType {
    fn new() -> AllCommandType { AllCommandType(None) }
}

impl CommandType for AllCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(std("_eq_all"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AllCommand(it.regs.to_vec(),self.0.clone())))
    }
    
    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&EqAllTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }   
}

pub struct AllCommand(Vec<Register>,Option<TimeTrial>);

impl AllCommand {
    fn enough_valid(&self, context: &PreImageContext) -> Result<bool,String> {
        for i in 1..self.0.len() {
            if !context.is_reg_valid(&self.0[i]) { return Ok(false); }
        }
        Ok(true)
    }

    fn any_size(&self, context: &PreImageContext) -> Option<usize> {
        for i in 1..self.0.len() {
            if let Some(size) = context.get_reg_size(&self.0[i]) {
                return Some(size);
            }
        }
        None
    }
}

impl Command for AllCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.0.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![regs]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if self.enough_valid(context)? && !context.is_last() {
            PreImagePrepare::Replace
        } else if let Some(size) = self.any_size(context) {
            PreImagePrepare::Keep(vec![(self.0[0].clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0[0].clone()]))
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = context.get_reg_size(&self.0[1]) {
            self.1.as_ref().map(|x| x.evaluate(size as f64/200.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

pub struct EqCommandType();

impl CommandType for EqCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("eq"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(EqCommand(sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct EqCommand(RegisterSignature,Vec<Register>);

impl EqCommand {
    fn build_instrs(&self, context: &mut PreImageContext) -> Result<Vec<Instruction>,String> {
        let mut out = vec![];
        let mut parts = vec![self.1[0].clone()];
        let short = self.0[1].iter().count() == 1;
        for ((_,vr_a),(_,vr_b)) in self.0[1].iter().zip(self.0[2].iter()) {
            let target = if short {
                self.1[0]
            } else {
                let part = context.new_register();
                parts.push(part.clone());
                part
            };
            let mut sigs = RegisterSignature::new();
            let mut cr = FullType::new_empty(MemberMode::Out);
            cr.add(ComplexPath::new_empty(),VectorRegisters::new(0,BaseType::BooleanType));
            sigs.add(cr);
            let mut cr = FullType::new_empty(MemberMode::In);
            cr.add(ComplexPath::new_empty().add_levels(vr_a.depth()),VectorRegisters::new(vr_a.depth(),vr_a.get_base().clone()));
            sigs.add(cr);
            let mut cr = FullType::new_empty(MemberMode::In);
            cr.add(ComplexPath::new_empty().add_levels(vr_b.depth()),VectorRegisters::new(vr_b.depth(),vr_b.get_base().clone()));
            sigs.add(cr);
            let mut regs = vec![target];
            regs.extend(vr_a.all_registers().iter().map(|x| self.1[*x].clone()));
            regs.extend(vr_b.all_registers().iter().map(|x| self.1[*x].clone()));
            let name = if vr_a.depth() == 0 { "_eq_shallow" } else { "_eq_compare" };
            out.push(Instruction::new(InstructionType::Call(Identifier::new("std",name),false,sigs,
                        vec![MemberDataFlow::Out,MemberDataFlow::In,MemberDataFlow::In]),regs));
        }
        if !short {
            let mut sigs = RegisterSignature::new();
            let mut cr = FullType::new_empty(MemberMode::Out);
            cr.add(ComplexPath::new_empty(),VectorRegisters::new(0,BaseType::BooleanType));
            for _ in 0..parts.len() {
                let mut cr = FullType::new_empty(MemberMode::In);
                cr.add(ComplexPath::new_empty(),VectorRegisters::new(0,BaseType::BooleanType));
            }
            sigs.add(cr);
            let mut flows = vec![MemberDataFlow::Out];
            flows.extend(parts.iter().map(|_| MemberDataFlow::In));
            out.push(Instruction::new(InstructionType::Call(Identifier::new("std","_eq_all"),false,sigs,flows),parts));
        }
        Ok(out)
    }
}

impl Command for EqCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Err(format!("compile-side command"))
    }

    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Replace(self.build_instrs(context)?))
    }
}

pub(super) fn library_eq_command(set: &mut CompLibRegister) -> Result<(),String> {
    set.push("eq",None,EqCommandType());
    set.push("_eq_shallow",Some(0),EqShallowCommandType::new());
    set.push("_eq_compare",Some(19),EqCompareCommandType::new());
    set.push("_eq_all",Some(20),AllCommandType::new());
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::test::{ xxx_test_config, compile };

    #[test]
    fn eq_smoke() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_verbose(3);
        let strings = compile(&config,"search:std/eq").expect("a");
        for s in &strings {
            print!("{}\n",s);
        }
    }
}
