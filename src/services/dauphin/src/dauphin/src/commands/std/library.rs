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

use crate::interp::InterpNatural;
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier, cbor_make_map, ComplexRegisters };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents, PreImageOutcome, Stream, PreImagePrepare, InterpValue };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use super::numops::library_numops_commands;
use super::eq::library_eq_command;
use super::assign::library_assign_commands;
use crate::cli::Config;
use crate::typeinf::MemberMode;
use crate::interp::{ CompilerLink, TimeTrialCommandType, trial_write, trial_signature, TimeTrial };

pub fn std_id() -> CommandSetId {
    CommandSetId::new("std",(0,0),0x43642F36EF881EFC)
}

pub(super) fn std(name: &str) -> Identifier {
    Identifier::new("std",name)
}

pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
    let p = context.payload("std","stream")?;
    Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
}

struct LenTimeTrial();

impl TimeTrialCommandType for LenTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn global_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = (t*100) as usize;
        trial_write(context,1,1,|_| 0);
        trial_write(context,2,1,|_| t);
        trial_write(context,3,t,|x| x*10);
        trial_write(context,4,t,|_| 10);
        trial_write(context,5,t*10,|x| x);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        let sig = trial_signature(&vec![(MemberMode::RValue,0),(MemberMode::RValue,2)]);
        let regs : Vec<Register> = (0..6).map(|x| Register(x)).collect();
        Ok(Box::new(LenCommand(sig,regs)))
    }
}

pub struct LenCommandType();

impl CommandType for LenCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("len"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(LenCommand(sig.clone(),it.regs.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<Vec<_>,_>>()?;
        Ok(Box::new(LenCommand(RegisterSignature::deserialize(&value[1],false,false)?,regs)))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&LenTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }
}

pub struct LenCommand(pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for LenCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        if let Some((_,ass)) = &self.0[1].iter().next() {
            let reg = ass.length_pos(ass.depth()-1)?;
            registers.copy(&self.1[0],&self.1[reg])?;
            Ok(())
        } else {
            Err("len on non-list".to_string())
        }
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![CborValue::Array(self.1.iter().map(|x| x.serialize()).collect()),self.0.serialize(false,false)?])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        if let Some((_,ass)) = &self.0[1].iter().next() {
            let reg = ass.length_pos(ass.depth()-1)?;
            Ok(if context.is_reg_valid(&self.1[reg]) {
                PreImagePrepare::Replace
            } else if let Some(a) = context.get_reg_size(&self.1[reg]) {
                PreImagePrepare::Keep(vec![(self.1[0].clone(),a)])
            } else {
                PreImagePrepare::Keep(vec![])
            })
        } else {
            Err("len on non-list".to_string())
        }
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Constant(vec![self.1[0]]))
    }
}

pub struct PrintRegsCommandType();

impl CommandType for PrintRegsCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(std("print_regs"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(PrintRegsCommand(it.regs.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<Vec<_>,_>>()?;
        Ok(Box::new(PrintRegsCommand(regs)))
    }
}

pub struct PrintRegsCommand(pub(crate) Vec<Register>);

impl Command for PrintRegsCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        for r in &self.0 {
            let v = StreamContents::Data(context.registers().get(r).borrow().get_shared()?.copy());
            print!("ADDING {:?}\n",v);
            std_stream(context)?.add(v);
        }
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![CborValue::Array(self.0.iter().map(|x| x.serialize()).collect())])
    }
}

fn print_value<T>(data: &[T], start: usize, len: usize) -> String where T: std::fmt::Display {
    let mut out = Vec::new();
    for index in start..start+len {
        out.push(data[index].to_string());
    }
    out.join(",")
}

fn print_bytes<T>(data: &[Vec<T>], start: usize, len: usize) -> String where T: std::fmt::Display {
    let mut out = vec![];
    for index in start..start+len {
        out.push(format!("[{}]",data[index].iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")));
    }
    out.join(",")
}

fn print_register(context: &mut InterpContext, reg: &Register, restrict: Option<(usize,usize)>) -> Result<String,String> {
    let value = context.registers().get(reg);
    let value = value.borrow().get_shared()?;
    let (start,len) = restrict.unwrap_or_else(|| { (0,value.len()) });
    Ok(match value.get_natural() {
        InterpNatural::Empty => { "[]".to_string() },
        InterpNatural::Numbers => { print_value(&value.to_rc_numbers()?.0, start, len) },
        InterpNatural::Indexes => { print_value(&value.to_rc_indexes()?.0, start, len) },
        InterpNatural::Boolean => { print_value(&value.to_rc_boolean()?.0, start, len) },
        InterpNatural::Strings => { print_value(&value.to_rc_strings()?.0, start, len) },
        InterpNatural::Bytes => { print_bytes(&value.to_rc_bytes()?.0, start, len) },
    })
}

fn print_base(context: &mut InterpContext, assignment: &VectorRegisters, regs: &[Register], restrict: Option<(usize,usize)>) -> Result<String,String> {
    let data_reg = assignment.data_pos();
    print_register(context,&regs[data_reg],restrict)
}

fn print_level(context: &mut InterpContext, assignment: &VectorRegisters, regs: &[Register], level_in: i64, restrict: Option<(usize,usize)>) -> Result<String,String> {
    if level_in > -1 {
        let level = level_in as usize;
        /* find registers for level */
        let offset_reg = assignment.offset_pos(level)?;
        let len_reg = assignment.length_pos(level)?;
        let starts = &context.registers().get_indexes(&regs[offset_reg])?;
        let lens = &context.registers().get_indexes(&regs[len_reg])?;
        let lens_len = lens.len();
        let (a,b) = restrict.unwrap_or((0,lens_len));
        let mut members = Vec::new();
        for index in a..a+b {
            members.push(print_level(context,assignment,regs,level_in-1,Some((starts[index],lens[index%lens_len])))?);
        }
        Ok(format!("{}",members.iter().map(|x| format!("[{}]",x)).collect::<Vec<_>>().join(",")))
    } else {
        print_base(context,assignment,regs,restrict)
    }
}

fn print_array(context: &mut InterpContext, assignment: &VectorRegisters, regs: &[Register]) -> Result<String,String> {
    let mut out = print_level(context,assignment,regs,assignment.depth() as i64-1,None)?;
    if out.len() == 0 { out = "-".to_string() }
    Ok(out)
}

fn print_complex(context: &mut InterpContext, assignment: &VectorRegisters, regs: &[Register], complex: &ComplexPath, is_complex: bool) -> Result<String,String> {
    if is_complex {
        Ok(format!("{}: {}",complex.to_string(),print_array(context,assignment,regs)?))
    } else {
        print_array(context,assignment,regs)
    }
}

fn print_vec(context: &mut InterpContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<String,String> {
    let mut out : Vec<String> = vec![];
    let is_complex = sig[0].iter().count() > 1;
    for (complex,a) in sig[0].iter() {
        out.push(print_complex(context,&a,regs,&complex,is_complex)?);
    }
    let mut out = out.join("; ");
    if is_complex { out = format!("{{ {} }}",out); }
    Ok(out)
}

pub struct PrintVecCommandType();

impl CommandType for PrintVecCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("print_vec"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(PrintVecCommand(sig.clone(),it.regs.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[0],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<Vec<_>,_>>()?;
        Ok(Box::new(PrintVecCommand(RegisterSignature::deserialize(&value[1],true,true)?,regs)))
    }
}

pub struct PrintVecCommand(pub(crate) RegisterSignature,pub(crate) Vec<Register>);

impl Command for PrintVecCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let v = StreamContents::String(print_vec(context,&self.0,&self.1)?);
        std_stream(context)?.add(v);
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![CborValue::Array(self.1.iter().map(|x| x.serialize()).collect()),self.0.serialize(true,true)?])
    }
}

pub struct AssertCommandType();

impl CommandType for AssertCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std("assert"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(AssertCommand(it.regs[0],it.regs[1])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AssertCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?)))
    }
}

pub struct AssertCommand(pub(crate) Register, pub(crate) Register);

impl Command for AssertCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = &registers.get_boolean(&self.0)?;
        let b = &registers.get_boolean(&self.1)?;
        for i in 0..a.len() {
            if a[i] != b[i%b.len()] {
                return Err(format!("assertion failed index={}!",i));
            }
        }
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.0.serialize(),self.1.serialize()])
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> {
        Ok(if context.is_reg_valid(&self.0) && context.is_reg_valid(&self.1) {
            PreImagePrepare::Replace
        } else if let Some(a) = context.get_reg_size(&self.0) {
            PreImagePrepare::Keep(vec![(self.0.clone(),a)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }
    
    fn preimage_post(&self, _context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Replace(vec![]))
    }
}

pub struct AlienateCommandType();

impl CommandType for AlienateCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(std("alienate"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(AlienateCommand(it.regs.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(AlienateCommand(vec![])))
    }
}

pub struct AlienateCommand(pub(crate) Vec<Register>);

impl Command for AlienateCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![])
    }
    
    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        for reg in self.0.iter() {
            context.set_reg_invalid(reg);
        }
        Ok(PreImageOutcome::Skip(vec![]))
    }
}

fn hint_reg(sig: &ComplexRegisters, regs: &[Register]) -> Result<Vec<Register>,String> {
    let mut out = vec![];
    for (_,vr) in sig.iter() {
        if vr.depth() > 0 {
            out.push(regs[vr.offset_pos(vr.depth()-1)?]);
        } else {
            out.push(regs[vr.data_pos()]);
        }
    }
    Ok(out)
}

pub struct GetSizeHintCommandType();

impl CommandType for GetSizeHintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(std("get_size_hint"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(GetSizeHintCommand(it.regs[0].clone(),hint_reg(&sig[1],&it.regs)?)))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, _value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Err(format!("cannot deseriailize size hints"))
    }
}

pub struct GetSizeHintCommand(Register,Vec<Register>);

impl Command for GetSizeHintCommand {
    fn execute(&self, _context: &mut InterpContext) -> Result<(),String> {
        Err(format!("cannot execute size hints"))
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Err(format!("cannot seriailize size hints"))
    }
    
    fn preimage(&self, context: &mut PreImageContext) -> Result<PreImageOutcome,String> {
        let mut out = vec![];
        for reg in self.1.iter() {
            out.push(context.get_reg_size(reg).unwrap_or(0));
        }
        context.context().registers().write(&self.0,InterpValue::Indexes(out));
        Ok(PreImageOutcome::Constant(vec![self.0.clone()]))
    }
}

// TODO ARRAY-proof!
pub struct PrintCommandType();

impl CommandType for PrintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(std("print"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_,_) = &it.itype {
            Ok(Box::new(PrintCommand(it.regs[0])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let reg = Register::deserialize(value[0])?;
        Ok(Box::new(PrintCommand(reg)))
    }
}

pub struct PrintCommand(Register);

impl Command for PrintCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = &registers.coerce_strings(&self.0)?;
        for s in a.iter() {
            std_stream(context)?.add(StreamContents::String(s.to_string()));
        }
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
       Ok(vec![self.0.serialize()])
    }    
}

pub fn make_library() -> Result<CommandSet,String> {
    let mut set = CommandSet::new(&std_id(),false);
    library_eq_command(&mut set)?;
    set.push("len",1,LenCommandType())?;
    set.push("print_regs",2,PrintRegsCommandType())?;
    set.push("print_vec",3,PrintVecCommandType())?;
    set.push("assert",4,AssertCommandType())?;
    set.push("alienate",13,AlienateCommandType())?;
    set.push("print",14,PrintCommandType())?;
    set.push("get_size_hint",15,GetSizeHintCommandType())?;
    set.add_header("std",include_str!("header.dp"));
    library_numops_commands(&mut set)?;
    library_assign_commands(&mut set)?;
    set.load_dynamic_data(include_bytes!("std-0.0.ddd"))?;
    Ok(set)
}
