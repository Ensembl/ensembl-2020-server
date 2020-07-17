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

use std::fmt;

use crate::util::DFloat;
use crate::lexer::LexerPosition;
use serde_cbor::Value as CborValue;
use dauphin_interp::command::{ Identifier };
use dauphin_interp::runtime::{ Register  };
use dauphin_interp::types::{ RegisterSignature, MemberMode, MemberDataFlow };
use dauphin_interp::util::cbor::{ cbor_int };

#[derive(Clone,Copy,PartialEq,Debug,Hash,Eq)]
pub enum InstructionSuperType {
    Pause,
    Nil,
    Copy,
    Append,
    Filter,
    Run,
    At,
    NumEq,
    ReFilter,
    Length,
    Add,
    SeqFilter,
    SeqAt,
    Const,
    NumberConst,
    BooleanConst,
    StringConst,
    BytesConst,
    Call,
    LineNumber
}

impl InstructionSuperType {
    pub fn serialize(&self) -> CborValue {
        CborValue::Integer(match self {
            InstructionSuperType::Pause => 0,
            InstructionSuperType::Nil => 1,
            InstructionSuperType::Copy => 2,
            InstructionSuperType::Append => 3,
            InstructionSuperType::Filter => 4,
            InstructionSuperType::Run => 5,
            InstructionSuperType::At => 6,
            InstructionSuperType::NumEq => 7,
            InstructionSuperType::ReFilter => 8,
            InstructionSuperType::Length => 9,
            InstructionSuperType::Add => 10,
            InstructionSuperType::SeqFilter => 11,
            InstructionSuperType::SeqAt => 12,
            InstructionSuperType::Const => 13,
            InstructionSuperType::NumberConst => 14,
            InstructionSuperType::BooleanConst => 15,
            InstructionSuperType::StringConst => 16,
            InstructionSuperType::BytesConst => 17,
            InstructionSuperType::Call => 18,
            InstructionSuperType::LineNumber => 19
        })
    }

    pub fn deserialize(value: &CborValue) -> Result<InstructionSuperType,String> {
        Ok(match cbor_int(value,Some(19))? {
            0 => InstructionSuperType::Pause,
            1 => InstructionSuperType::Nil,
            2 => InstructionSuperType::Copy,
            3 => InstructionSuperType::Append,
            4 => InstructionSuperType::Filter,
            5 => InstructionSuperType::Run,
            6 => InstructionSuperType::At,
            7 => InstructionSuperType::NumEq,
            8 => InstructionSuperType::ReFilter,
            9 => InstructionSuperType::Length,
            10 => InstructionSuperType::Add,
            11 => InstructionSuperType::SeqFilter,
            12 => InstructionSuperType::SeqAt,
            13 => InstructionSuperType::Const,
            14 => InstructionSuperType::NumberConst,
            15 => InstructionSuperType::BooleanConst,
            16 => InstructionSuperType::StringConst,
            17 => InstructionSuperType::BytesConst,
            18 => InstructionSuperType::Call,
            19 => InstructionSuperType::LineNumber,
            _ => Err(format!("impossible in IntstructionSuperType deserialize"))?
        })
    }
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub enum InstructionType {
    Pause(bool),
    Nil,
    Alias,
    Copy,
    Append,
    Square,
    FilterSquare,
    RefSquare,
    Star,
    At,
    Filter,
    Run,
    NumEq,
    ReFilter,
    Length,
    Add,
    SeqFilter,
    SeqAt,
    Const(Vec<usize>),
    NumberConst(DFloat),
    BooleanConst(bool),
    StringConst(String),
    BytesConst(Vec<u8>),
    CtorStruct(Identifier),
    CtorEnum(Identifier,String),
    SValue(Identifier,String),
    RefSValue(Identifier,String),
    EValue(Identifier,String),
    RefEValue(Identifier,String),
    FilterEValue(Identifier,String),
    ETest(Identifier,String),
    Proc(Identifier,Vec<MemberMode>),
    Operator(Identifier),
    Call(Identifier,bool,RegisterSignature,Vec<MemberDataFlow>),
    LineNumber(LexerPosition)
}

impl InstructionType {
    pub fn supertype(&self) -> Result<InstructionSuperType,String> {
        Ok(match self {
            InstructionType::Nil => InstructionSuperType::Nil,
            InstructionType::Pause(_) => InstructionSuperType::Pause,
            InstructionType::Copy => InstructionSuperType::Copy,
            InstructionType::Append => InstructionSuperType::Append,
            InstructionType::Filter => InstructionSuperType::Filter,
            InstructionType::Run => InstructionSuperType::Run,
            InstructionType::At => InstructionSuperType::At,
            InstructionType::NumEq => InstructionSuperType::NumEq,
            InstructionType::ReFilter => InstructionSuperType::ReFilter,
            InstructionType::Length => InstructionSuperType::Length,
            InstructionType::Add => InstructionSuperType::Add,
            InstructionType::SeqFilter => InstructionSuperType::SeqFilter,
            InstructionType::SeqAt => InstructionSuperType::SeqAt,
            InstructionType::Const(_) => InstructionSuperType::Const,
            InstructionType::NumberConst(_) => InstructionSuperType::NumberConst,
            InstructionType::BooleanConst(_) => InstructionSuperType::BooleanConst,
            InstructionType::StringConst(_) => InstructionSuperType::StringConst,
            InstructionType::BytesConst(_) => InstructionSuperType::BytesConst,
            InstructionType::Call(_,_,_,_) => InstructionSuperType::Call,
            InstructionType::LineNumber(_) => InstructionSuperType::LineNumber,
            _ => Err(format!("instruction has no supertype"))?
        })
    }

    pub fn get_name(&self) -> Vec<String> {
        let call = match self {
            InstructionType::Pause(_) => "pause",
            InstructionType::Nil => "nil",
            InstructionType::Alias => "alias",
            InstructionType::Copy => "copy",
            InstructionType::Append => "append",
            InstructionType::Square => "square",
            InstructionType::FilterSquare => "filtersquare",
            InstructionType::RefSquare => "refsquare",
            InstructionType::Star => "star",
            InstructionType::At => "at",
            InstructionType::Filter => "filter",
            InstructionType::Run => "run",
            InstructionType::NumEq => "numeq",
            InstructionType::ReFilter => "refilter",
            InstructionType::Length => "length",
            InstructionType::Add => "add",
            InstructionType::SeqFilter => "seqfilter",
            InstructionType::SeqAt => "seqat",
            InstructionType::NumberConst(_) => "number",
            InstructionType::BooleanConst(_) => "bool",
            InstructionType::StringConst(_) => "string",
            InstructionType::BytesConst(_) => "bytes",
            InstructionType::CtorStruct(_) => "struct",
            InstructionType::CtorEnum(_,_) => "enum",
            InstructionType::SValue(_,_) => "svalue",
            InstructionType::RefSValue(_,_) => "refsvalue",
            InstructionType::EValue(_,_) => "evalue",
            InstructionType::FilterEValue(_,_) => "frevalue",
            InstructionType::RefEValue(_,_) => "refevalue",
            InstructionType::ETest(_,_) => "etest",
            InstructionType::Proc(_,_) => "proc",
            InstructionType::Operator(_) => "oper",
            InstructionType::Call(_,_,_,_) => "call",
            InstructionType::Const(_) => "const",
            InstructionType::LineNumber(_) => "line",
        }.to_string();
        let mut out = vec![call.clone()];
        if let Some(prefixes) = match self {
            InstructionType::Pause(true) => Some(vec!["force".to_string()]),
            InstructionType::CtorStruct(name) => Some(vec![name.to_string()]),
            InstructionType::CtorEnum(name,branch) => Some(vec![name.to_string(),branch.to_string()]),
            InstructionType::SValue(name,field) => Some(vec![name.to_string(),field.to_string()]),
            InstructionType::EValue(name,branch) => Some(vec![name.to_string(),branch.to_string()]),
            InstructionType::ETest(name,branch) => Some(vec![name.to_string(),branch.to_string()]),
            InstructionType::Operator(name) => {
                Some(vec![name.name().to_string()]) // XXX module
            },
            InstructionType::Proc(name,modes) =>  {
                let mut more = vec![];
                more.push(name.name().to_string()); // XXX module
                more.extend(modes.iter().map(|x| x.to_string()).collect::<Vec<_>>());
                Some(more)
            },            
            InstructionType::Call(name,impure,types,_) => {
                let mut name = name.to_string();
                if *impure { name.push_str("/i"); }
                let mut more = vec![name.to_string()]; // XXX module
                more.extend(types.iter().map(|x| x.to_string()).collect::<Vec<_>>());
                Some(more)
            },
            _ => None
        } {
            out[0] = format!("{}:{}",call,prefixes.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(":"));
        };
        
        if let Some(suffix) = match self {
            InstructionType::NumberConst(n) => Some(n.as_f64().to_string()),
            InstructionType::BooleanConst(b) => Some(b.to_string()),
            InstructionType::StringConst(s) => Some(format!("\"{}\"",s.to_string())),
            InstructionType::BytesConst(b) => Some(format!("\'{}\'",hex::encode(b))),
            InstructionType::Const(c) => Some(c.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")),
            InstructionType::LineNumber(pos) => Some(pos.to_string()),
            _ => None
        } {
            out.push(suffix);
        }
        out
    }

    pub fn self_justifying_call(&self) -> bool {
        match self {
            InstructionType::Call(_,impure,_,_) => *impure,
            InstructionType::LineNumber(_) => true,
            InstructionType::Pause(_) => true,
            _ => false
        }
    }

    pub fn out_only_registers(&self) -> Vec<usize> {
        match self {
            InstructionType::LineNumber(_) => vec![],
            InstructionType::Pause(_) => vec![],

            InstructionType::Call(_,_,sigs,dataflow) => {
                let mut out = Vec::new();
                let mut reg_offset = 0;
                for (i,sig) in sigs.iter().enumerate() {
                    let num_regs = sig.iter().map(|x| x.1.register_count()).sum();
                    match dataflow[i] {
                        MemberDataFlow::Out => {
                            for j in 0..num_regs {
                                out.push(reg_offset+j);
                            }
                        },
                        MemberDataFlow::In | MemberDataFlow::InOut => {}
                    }
                    reg_offset += num_regs;
                }
                out
            },

            InstructionType::Add | InstructionType::Append => vec![],
            _ => vec![0]
        }
    }

    pub fn out_registers(&self) -> Vec<usize> {
        match self {
            InstructionType::LineNumber(_) => vec![],
            InstructionType::Pause(_) => vec![],

            InstructionType::Call(_,_,sigs,dataflow) => {
                let mut out = Vec::new();
                let mut reg_offset = 0;
                for (i,sig) in sigs.iter().enumerate() {
                    let num_regs = sig.iter().map(|x| x.1.register_count()).sum();
                    match dataflow[i] {
                        MemberDataFlow::Out | MemberDataFlow::InOut => {
                            for j in 0..num_regs {
                                out.push(reg_offset+j);
                            }
                        },
                        MemberDataFlow::In => {}
                    }
                    reg_offset += num_regs;
                }
                out
            },

            _ => vec![0]
        }
    }
}

#[derive(Clone,PartialEq)]
pub struct Instruction {
    pub itype: InstructionType,
    pub regs: Vec<Register>
}

fn fmt_instr2(f: &mut fmt::Formatter<'_>, opcode: &str, regs: &Vec<Register>, more: &[String]) -> fmt::Result {
    let mut regs : Vec<String> = regs.iter().map(|x| format!("{:?}",x)).collect();
    if more.len() > 0 { regs.push("".to_string()); }
    write!(f,"#{} {}{};\n",opcode,regs.join(" "),more.join(" "))?;
    Ok(())
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = self.itype.get_name();
        fmt_instr2(f,&args[0],&self.regs,&args[1..])?;
        Ok(())
    }
}

impl Instruction {
    pub fn new(itype: InstructionType, regs: Vec<Register>) -> Instruction {
        Instruction { itype, regs }
    }
}
