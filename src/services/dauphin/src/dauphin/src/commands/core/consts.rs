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

use std::convert::TryInto;
use crate::interp::InterpValue;
use crate::interp::{ InterpContext, Command, CommandSchema, CommandType, CommandTrigger, CommandSet, PreImageOutcome, PreImagePrepare, TimeTrialCommandType, TimeTrial };
use crate::model::Register;
use crate::model::{ cbor_int, cbor_string, cbor_make_map, cbor_map };
use crate::generate::{ Instruction, InstructionType, InstructionSuperType, PreImageContext };
use serde_cbor::Value as CborValue;
use crate::cli::Config;
use crate::interp::CompilerLink;

// XXX factor
macro_rules! force_branch {
    ($value:expr,$ty:ident,$branch:ident) => {
        if let $ty::$branch(v) = $value {
            Ok(v)
        } else {
            Err("Cannot extract".to_string())
        }?
    };
}

struct NumberConstTimeTrial();

impl TimeTrialCommandType for NumberConstTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,1) }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumberConstCommand(Register(0),42.,0.)))
    }
}

pub struct NumberConstCommandType(f64);

impl NumberConstCommandType {
    pub fn new() -> NumberConstCommandType { NumberConstCommandType(1.) }
}

impl CommandType for NumberConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::NumberConst),
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumberConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,NumberConst).as_f64(),self.0)))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(NumberConstCommand(Register::deserialize(&value[0])?,*force_branch!(value[1],CborValue,Float),self.0)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&NumberConstTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = TimeTrial::deserialize(&t[0])?.evaluate(1.);
        Ok(())
    }
}

pub struct NumberConstCommand(Register,f64,f64);

impl Command for NumberConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Numbers(vec![self.1]));
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),CborValue::Float(self.1)]))
    }

    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(PreImagePrepare::Replace)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.2 }
}

struct ConstTimeTrial();

impl TimeTrialCommandType for ConstTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        Ok(Box::new(ConstCommand(Register(0),num,None)))
    }
}

pub struct ConstCommandType(Option<TimeTrial>);

impl ConstCommandType {
    pub fn new() -> ConstCommandType { ConstCommandType(None) }
}

impl CommandType for ConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::Const)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(ConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,Const).to_vec(),self.0.clone())))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let v = force_branch!(&value[1],CborValue,Array);
        let v = v.iter().map(|x| { Ok(*force_branch!(x,CborValue,Integer) as usize) }).collect::<Result<Vec<usize>,String>>()?;
        Ok(Box::new(ConstCommand(Register::deserialize(&value[0])?,v,self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&ConstTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct ConstCommand(Register,Vec<usize>,Option<TimeTrial>);

impl Command for ConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Indexes(self.1.to_vec()));
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let v = self.1.iter().map(|x| CborValue::Integer(*x as i128)).collect();
        Ok(Some(vec![self.0.serialize(),CborValue::Array(v)]))
    }

    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(PreImagePrepare::Replace)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.2.as_ref().map(|x| x.evaluate(self.1.len() as f64/100.)).unwrap_or(1.) }
}

struct BooleanConstTimeTrial();

impl TimeTrialCommandType for BooleanConstTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,1) }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BooleanConstCommand(Register(0),false,0.)))
    }
}

pub struct BooleanConstCommandType(f64);

impl BooleanConstCommandType {
    pub fn new() -> BooleanConstCommandType { BooleanConstCommandType(1.) }
}

impl CommandType for BooleanConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::BooleanConst)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BooleanConstCommand(it.regs[0],force_branch!(it.itype,InstructionType,BooleanConst),self.0)))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BooleanConstCommand(Register::deserialize(&value[0])?,*force_branch!(value[1],CborValue,Bool),self.0)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&BooleanConstTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = TimeTrial::deserialize(&t[0])?.evaluate(1.);
        Ok(())
    }
}

pub struct BooleanConstCommand(Register,bool,f64);

impl Command for BooleanConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Boolean(vec![self.1]));
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),CborValue::Bool(self.1)]))
    }

    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(PreImagePrepare::Replace)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.2 }
}

struct StringTimeTrial();

impl TimeTrialCommandType for StringTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let x = "x".repeat((t*100) as usize);
        Ok(Box::new(StringConstCommand(Register(0),x,None)))
    }
}

pub struct StringConstCommandType(Option<TimeTrial>);

impl StringConstCommandType {
    pub fn new() -> StringConstCommandType { StringConstCommandType(None) }
}

impl CommandType for StringConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::StringConst)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(StringConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,StringConst).to_string(),self.0.clone())))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let v = force_branch!(value[1],CborValue,Text).to_string();
        Ok(Box::new(StringConstCommand(Register::deserialize(&value[0])?,v,self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&StringTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct StringConstCommand(Register,String,Option<TimeTrial>);

impl Command for StringConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Strings(vec![self.1.to_string()]));
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),CborValue::Text(self.1.to_string())]))
    }

    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(PreImagePrepare::Replace)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.2.as_ref().map(|x| x.evaluate(self.1.len() as f64/100.)).unwrap_or(1.) }
}

struct BytesTimeTrial();

impl TimeTrialCommandType for BytesTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,10) }

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let x = vec![3].repeat((t*100) as usize);
        Ok(Box::new(BytesConstCommand(Register(0),x,None)))
    }
}

pub struct BytesConstCommandType(Option<TimeTrial>);

impl BytesConstCommandType {
    pub fn new() -> BytesConstCommandType { BytesConstCommandType(None) }
}

impl CommandType for BytesConstCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::BytesConst)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(BytesConstCommand(it.regs[0],force_branch!(&it.itype,InstructionType,BytesConst).to_vec(),self.0.clone())))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let v = force_branch!(value[1],CborValue,Bytes).to_vec();
        Ok(Box::new(BytesConstCommand(Register::deserialize(&value[0])?,v,self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&BytesTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct BytesConstCommand(Register,Vec<u8>,Option<TimeTrial>);

impl Command for BytesConstCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Bytes(vec![self.1.to_vec()]));
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),CborValue::Bytes(self.1.to_vec())]))
    }
    
    fn simple_preimage(&self, _context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(PreImagePrepare::Replace)
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.0]))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.2.as_ref().map(|x| x.evaluate(self.1.len() as f64/100.)).unwrap_or(1.) }
}

struct LineNumberTimeTrial();

impl TimeTrialCommandType for LineNumberTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (0,1) }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(LineNumberCommand("x".to_string(),42,1.)))
    }
}

pub struct LineNumberCommandType(f64);

impl LineNumberCommandType {
    fn new() -> LineNumberCommandType { LineNumberCommandType(1.) }
}

impl CommandType for LineNumberCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::LineNumber)
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        let (file,line) = if let InstructionType::LineNumber(file,line) = &it.itype {
            (file,line)
        } else {
            return Err(format!("malformatted cbor"));
        };
        Ok(Box::new(LineNumberCommand(file.to_string(),(*line).try_into().unwrap_or(0),self.0)))
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(LineNumberCommand(cbor_string(&value[0])?,cbor_int(&value[1],None)? as u32,self.0)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&LineNumberTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = TimeTrial::deserialize(&t[0])?.evaluate(1.);
        Ok(())
    }
}

pub struct LineNumberCommand(String,u32,f64);

impl Command for LineNumberCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.set_line_number(&self.0,self.1);
        Ok(())
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![CborValue::Text(self.0.to_string()),CborValue::Integer(self.1 as i128)]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        context.context_mut().set_line_number(&self.0,self.1);
        Ok(PreImagePrepare::Keep(vec![]))
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Err(format!("preimage impossible on line-number command"))
    }

    fn execution_time(&self, _context: &PreImageContext) -> f64 { self.2 }
}

pub(super) fn const_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("number",0,NumberConstCommandType::new())?;
    set.push("const",1,ConstCommandType::new())?;
    set.push("boolean",2,BooleanConstCommandType::new())?;
    set.push("string",3,StringConstCommandType::new())?;
    set.push("bytes",4,BytesConstCommandType::new())?;
    set.push("linenumber",17,LineNumberCommandType::new())?;
    Ok(())
}