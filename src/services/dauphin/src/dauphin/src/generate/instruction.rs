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

use crate::model::{ DefStore, Register, RegisterSignature, Identifier, cbor_int };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, InstructionConstraint, MemberMode, MemberDataFlow };
use serde_cbor::Value as CborValue;

fn placeholder(ref_: bool) -> ArgumentConstraint {
    if ref_ {
        ArgumentConstraint::Reference(ArgumentExpressionConstraint::Placeholder(String::new()))
    } else {
        ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))
    }
}

fn array(ref_: bool) -> ArgumentConstraint {
    if ref_ {
        ArgumentConstraint::Reference(ArgumentExpressionConstraint::Vec(Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))
    } else {
        ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Vec(Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))
    }
}

fn fixed(bt: BaseType) -> ArgumentConstraint {
    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(bt))
}

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
            _ => Err(format!("impossiblr in IntstructionSuperType deserialize"))?
        })
    }
}

#[derive(Clone,PartialEq,Debug)]
pub enum InstructionType {
    Pause,
    Nil,
    Alias,
    Copy,
    List,
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
    NumberConst(f64),
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
    LineNumber(String,u32)
}

impl InstructionType {
    pub fn supertype(&self) -> Result<InstructionSuperType,String> {
        Ok(match self {
            InstructionType::Nil => InstructionSuperType::Nil,
            InstructionType::Pause => InstructionSuperType::Pause,
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
            InstructionType::LineNumber(_,_) => InstructionSuperType::LineNumber,
            _ => Err(format!("instruction has no supertype"))?
        })
    }

    pub fn get_name(&self) -> Vec<String> {
        let call = match self {
            InstructionType::Pause => "pause",
            InstructionType::Nil => "nil",
            InstructionType::Alias => "alias",
            InstructionType::Copy => "copy",
            InstructionType::List => "list",
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
            InstructionType::LineNumber(_,_) => "line",
        }.to_string();
        let mut out = vec![call.clone()];
        if let Some(prefixes) = match self {
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
            InstructionType::NumberConst(n) => Some(n.to_string()),
            InstructionType::BooleanConst(b) => Some(b.to_string()),
            InstructionType::StringConst(s) => Some(format!("\"{}\"",s.to_string())),
            InstructionType::BytesConst(b) => Some(format!("\'{}\'",hex::encode(b))),
            InstructionType::Const(c) => Some(c.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")),
            InstructionType::LineNumber(file,line) => Some(format!("\"{}\" {}",file,line)),
            _ => None
        } {
            out.push(suffix);
        }
        out
    }

    pub fn self_justifying_call(&self) -> bool {
        match self {
            InstructionType::Call(_,impure,_,_) => *impure,
            InstructionType::LineNumber(_,_) => true,
            InstructionType::Pause => true,
            _ => false
        }
    }

    pub fn out_only_registers(&self) -> Vec<usize> {
        match self {
            InstructionType::LineNumber(_,_) => vec![],
            InstructionType::Pause => vec![],

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
            InstructionType::LineNumber(_,_) => vec![],
            InstructionType::Pause => vec![],

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

    pub fn get_constraints(&self, defstore: &DefStore) -> Result<Vec<ArgumentConstraint>,String> {
        match self {
            InstructionType::CtorStruct(identifier) => {
                let exprdecl = defstore.get_struct_id(identifier)?;
                let intypes = exprdecl.get_member_types();
                let mut out = vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::StructType(identifier.clone())))];
                out.extend(intypes.iter().map(|t| ArgumentConstraint::NonReference(t.to_argumentexpressionconstraint())));
                Ok(out)
            },

            InstructionType::CtorEnum(identifier,branch) => {
                let exprdecl = defstore.get_enum_id(identifier)?;
                let intype = exprdecl.get_branch_type(branch).ok_or_else(|| format!("No such enum branch {:?}",branch))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone()))),
                    ArgumentConstraint::NonReference(intype.to_argumentexpressionconstraint())
                ])
            },

            InstructionType::SValue(identifier,field) => {
                let exprdecl = defstore.get_struct_id(identifier)?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::StructType(identifier.clone())))
                ])
            },

            InstructionType::RefSValue(identifier,field) => {
                let exprdecl = defstore.get_struct_id(identifier)?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::Reference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::Reference(ArgumentExpressionConstraint::Base(BaseType::StructType(identifier.clone())))
                ])
            },

            InstructionType::EValue(identifier,field) => {
                let exprdecl = defstore.get_enum_id(identifier)?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
                ])
            },

            InstructionType::FilterEValue(identifier,_) => {
                Ok(vec![
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
                ])
            },

            InstructionType::RefEValue(identifier,field) => {
                let exprdecl = defstore.get_enum_id(identifier)?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::Reference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::Reference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
                ])
            },

            InstructionType::ETest(identifier,_) => {
                Ok(vec![
                    fixed(BaseType::BooleanType),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(identifier.clone())))
                ])
            },

            InstructionType::Proc(identifier,modes) => {
                let procdecl = defstore.get_proc_id(identifier)?;
                let signature = procdecl.get_signature();
                let mut arguments = Vec::new();
                let mut member_index = 0;
                let members : Vec<_> = signature.each_member().collect();
                for mode in modes {
                    let constraint = match mode {
                        MemberMode::RValue | MemberMode::LValue => {
                            member_index += 1;
                            members[member_index-1].to_argumentconstraint()
                        },
                        MemberMode::FValue => ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType))
                    };
                    arguments.push(constraint);
                }
                Ok(arguments)
            },

            InstructionType::Operator(identifier) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_func_id(identifier)?;
                let signature = exprdecl.get_signature();
                for member_constraint in signature.each_member() {
                    out.push(member_constraint.to_argumentconstraint());
                }
                Ok(out)
            },

            InstructionType::Nil   => Ok(vec![placeholder(false)]),
            InstructionType::Alias => Ok(vec![placeholder(true),placeholder(false)]),
            InstructionType::Copy =>  Ok(vec![placeholder(false),placeholder(false)]),
            InstructionType::List => Ok(vec![array(false)]),
            InstructionType::Append => Ok(vec![placeholder(false),placeholder(false)]),
            InstructionType::Square => Ok(vec![placeholder(false),array(false)]),
            InstructionType::RefSquare => Ok(vec![placeholder(true),array(true)]),
            InstructionType::FilterSquare => Ok(vec![fixed(BaseType::NumberType),array(false)]),
            InstructionType::Star => Ok(vec![array(false),placeholder(false)]),
            InstructionType::At => Ok(vec![fixed(BaseType::NumberType),placeholder(false)]),
            InstructionType::Filter => Ok(vec![placeholder(false),placeholder(false),fixed(BaseType::BooleanType)]),
            InstructionType::Run => Ok(vec![fixed(BaseType::NumberType),fixed(BaseType::NumberType),fixed(BaseType::NumberType)]),
            InstructionType::NumberConst(_) | InstructionType::Const(_) => Ok(vec![fixed(BaseType::NumberType)]),
            InstructionType::BooleanConst(_) => Ok(vec![fixed(BaseType::BooleanType)]),
            InstructionType::StringConst(_) => Ok(vec![fixed(BaseType::StringType)]),
            InstructionType::BytesConst(_) => Ok(vec![fixed(BaseType::BytesType)]),
            InstructionType::ReFilter => Ok(vec![fixed(BaseType::NumberType),fixed(BaseType::NumberType),fixed(BaseType::NumberType)]),

            InstructionType::LineNumber(_,_) |
            InstructionType::Pause |
            InstructionType::NumEq |
            InstructionType::Length |
            InstructionType::Add |
            InstructionType::SeqFilter |
            InstructionType::SeqAt |
            InstructionType::Call(_,_,_,_) =>
                Ok(vec![]),
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

    pub fn get_constraint(&self, defstore: &DefStore) -> Result<InstructionConstraint,String> {
        let mut out = Vec::new();
        for (i,c) in self.itype.get_constraints(defstore)?.drain(..).enumerate() {
            out.push((c,self.regs[i]));
        }
        Ok(InstructionConstraint::new(&out))
    }
}
