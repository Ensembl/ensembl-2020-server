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
use crate::model::{ Register, VectorRegisters, RegisterSignature, cbor_array, ComplexPath, Identifier };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, CommandSetId, InterpContext, StreamContents };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use super::numops::library_numops_commands;
use super::eq::library_eq_command;
use super::assign::library_assign_commands;

pub(super) fn std(name: &str) -> Identifier {
    Identifier::new("std",name)
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
            context.stream_add(v);
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
        context.stream_add(v);
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
}

pub fn make_library() -> Result<CommandSet,String> {
    let set_id = CommandSetId::new("std",(0,0),0xA5E0F89826A426E7);
    let mut set = CommandSet::new(&set_id);
    library_eq_command(&mut set)?;
    set.push("len",1,LenCommandType())?;
    set.push("print_regs",2,PrintRegsCommandType())?;
    set.push("print_vec",3,PrintVecCommandType())?;
    set.push("assert",4,AssertCommandType())?;
    library_numops_commands(&mut set)?;
    library_assign_commands(&mut set)?;
    Ok(set)
}
