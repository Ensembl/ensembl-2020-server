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
use crate::interp::{ 
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, PreImagePrepare, TimeTrialCommandType, TimeTrial, 
    CompLibRegister
};
use crate::lexer::LexerPosition;
use dauphin_interp_common::common::{ cbor_int, cbor_string, cbor_make_map, cbor_map, Register, InterpCommand, CommandDeserializer };
use crate::model::DFloat;
use dauphin_interp_common::interp::{ InterpLibRegister, InterpValue, InterpContext };
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

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::NumberConst(DFloat::new(42.)),vec![Register(0)]))
    }
}

pub struct NumberConstDeserializer();

impl CommandDeserializer for NumberConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((0,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(NumberConstInterpCommand(Register::deserialize(&value[0])?,*force_branch!(value[1],CborValue,Float)))) 
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

pub struct NumberConstInterpCommand(Register,f64);

impl InterpCommand for NumberConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Numbers(vec![self.1]));
        Ok(())
    }
}

pub struct NumberConstCommand(Register,f64,f64);

impl Command for NumberConstCommand {
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

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let t = t*100;
        let num : Vec<usize> = (0..t).map(|x| x as usize).collect();
        Ok(Instruction::new(InstructionType::Const(num),vec![Register(0)]))
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

pub struct ConstDeserializer();

impl CommandDeserializer for ConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((1,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let v = force_branch!(&value[1],CborValue,Array);
        let v = v.iter().map(|x| { Ok(*force_branch!(x,CborValue,Integer) as usize) }).collect::<Result<Vec<usize>,String>>()?;
        Ok(Box::new(ConstInterpCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct ConstInterpCommand(Register,Vec<usize>);

impl InterpCommand for ConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Indexes(self.1.to_vec()));
        Ok(())
    }
}

pub struct ConstCommand(Register,Vec<usize>,Option<TimeTrial>);

impl Command for ConstCommand {
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

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::BooleanConst(false),vec![Register(0)]))
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

pub struct BooleanConstDeserializer();

impl CommandDeserializer for BooleanConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((2,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(BooleanConstInterpCommand(Register::deserialize(&value[0])?,*force_branch!(value[1],CborValue,Bool))))
    }
}

pub struct BooleanConstInterpCommand(Register,bool);

impl InterpCommand for BooleanConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Boolean(vec![self.1]));
        Ok(())
    }
}

pub struct BooleanConstCommand(Register,bool,f64);

impl Command for BooleanConstCommand {
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

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let x = "x".repeat((t*100) as usize);
        Ok(Instruction::new(InstructionType::StringConst(x),vec![Register(0)]))
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

pub struct StringConstDeserializer();

impl CommandDeserializer for StringConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((3,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let v = force_branch!(value[1],CborValue,Text).to_string();
        Ok(Box::new(StringConstInterpCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct StringConstInterpCommand(Register,String);

impl InterpCommand for StringConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Strings(vec![self.1.to_string()]));
        Ok(())
    }
}

pub struct StringConstCommand(Register,String,Option<TimeTrial>);

impl Command for StringConstCommand {
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

    fn timetrial_make_command(&self, t: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        let x = vec![3].repeat((t*100) as usize);
        Ok(Instruction::new(InstructionType::BytesConst(x),vec![Register(0)]))
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

pub struct BytesConstDeserializer();

impl CommandDeserializer for BytesConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((4,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let v = force_branch!(value[1],CborValue,Bytes).to_vec();
        Ok(Box::new(BytesConstInterpCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct BytesConstInterpCommand(Register,Vec<u8>);

impl InterpCommand for BytesConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Bytes(vec![self.1.to_vec()]));
        Ok(())
    }
}

pub struct BytesConstCommand(Register,Vec<u8>,Option<TimeTrial>);

impl Command for BytesConstCommand {
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

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Instruction,String> {
        Ok(Instruction::new(InstructionType::LineNumber(LexerPosition::new("x",42,0,None)),vec![]))
    }
}

pub struct LineNumberCommandType();

impl LineNumberCommandType {
    fn new() -> LineNumberCommandType { LineNumberCommandType() }
}

impl CommandType for LineNumberCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Instruction(InstructionSuperType::LineNumber)
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        let pos = if let InstructionType::LineNumber(pos) = &it.itype {
            pos
        } else {
            return Err(format!("malformatted cbor"));
        };
        Ok(Box::new(LineNumberCommand(pos.filename().to_string(),pos.line().try_into().unwrap_or(0))))
    }
}

pub struct LineNumberDeserializer();

impl CommandDeserializer for LineNumberDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((17,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(LineNumberInterpCommand(cbor_string(&value[0])?,cbor_int(&value[1],None)? as u32)))
    }
}


pub struct LineNumberInterpCommand(String,u32);

impl InterpCommand for LineNumberInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.set_line_number(&self.0,self.1);
        Ok(())
    }
}

pub struct LineNumberCommand(String,u32);

impl Command for LineNumberCommand {
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

    fn execution_time(&self, _context: &PreImageContext) -> f64 { 0. }
}

pub(super) fn const_commands(set: &mut CompLibRegister) -> Result<(),String> {
    set.push("number",Some(0),NumberConstCommandType::new());
    set.push("const",Some(1),ConstCommandType::new());
    set.push("boolean",Some(2),BooleanConstCommandType::new());
    set.push("string",Some(3),StringConstCommandType::new());
    set.push("bytes",Some(4),BytesConstCommandType::new());
    set.push("linenumber",Some(17),LineNumberCommandType::new());
    Ok(())
}

pub(super) fn const_commands_interp(set: &mut InterpLibRegister) -> Result<(),String> {
    set.push(NumberConstDeserializer());
    set.push(ConstDeserializer());
    set.push(BooleanConstDeserializer());
    set.push(StringConstDeserializer());
    set.push(BytesConstDeserializer());
    set.push(LineNumberDeserializer());
    Ok(())
}