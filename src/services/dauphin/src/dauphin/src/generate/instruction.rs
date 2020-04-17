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

use crate::model::{ DefStore, Register, RegisterSignature };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, InstructionConstraint, MemberMode };

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
    Call
}

#[derive(Clone,PartialEq,Debug)]
pub enum InstructionType {
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
    CtorStruct(String),
    CtorEnum(String,String),
    SValue(String,String),
    RefSValue(String,String),
    EValue(String,String),
    RefEValue(String,String),
    FilterEValue(String,String),
    ETest(String,String),
    Proc(String,Vec<MemberMode>),
    Operator(String),
    Call(String,bool,RegisterSignature)
}

impl InstructionType {
    pub fn get_name(&self) -> Vec<String> {
        let mut out = vec![match self {
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
            InstructionType::Call(_,_,_) => "call",
            InstructionType::Const(_) => "const",
        }.to_string()];
        if let Some(prefixes) = match self {
            InstructionType::CtorStruct(name) => Some(vec![name.to_string()]),
            InstructionType::CtorEnum(name,branch) => Some(vec![name.to_string(),branch.to_string()]),
            InstructionType::SValue(name,field) => Some(vec![name.to_string(),field.to_string()]),
            InstructionType::EValue(name,branch) => Some(vec![name.to_string(),branch.to_string()]),
            InstructionType::ETest(name,branch) => Some(vec![name.to_string(),branch.to_string()]),
            InstructionType::Operator(name) => Some(vec![name.to_string()]),
            InstructionType::Proc(name,modes) =>  {
                let mut out = vec![name.to_string()];
                out.extend(modes.iter().map(|x| x.to_string()).collect::<Vec<_>>());
                Some(out)
            },            
            InstructionType::Call(name,impure,types) => {
                let mut name = name.to_string();
                if *impure { name.push_str("/i"); }
                let mut out = vec![name.to_string()];
                out.extend(types.iter().map(|x| x.to_string()).collect::<Vec<_>>());
                Some(out)
            },
            _ => None
        } {
            out[0] = format!("{}:{}",out[0],prefixes.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(":"));
        };
        if let Some(suffix) = match self {
            InstructionType::NumberConst(n) => Some(n.to_string()),
            InstructionType::BooleanConst(b) => Some(b.to_string()),
            InstructionType::StringConst(s) => Some(format!("\"{}\"",s.to_string())),
            InstructionType::BytesConst(b) => Some(format!("\'{}\'",hex::encode(b))),
            InstructionType::Const(c) => Some(c.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ")),
            _ => None
        } {
            out.push(suffix);
        }
        out
    }

    pub fn self_justifying_call(&self) -> bool {
        match self {
            InstructionType::Call(_,impure,_) => *impure,
            _ => false
        }
    }

    pub fn changing_registers(&self) -> Vec<usize> {
        match self {
            InstructionType::Star |
            InstructionType::Alias |
            InstructionType::List |
            InstructionType::Square |
            InstructionType::RefSquare |
            InstructionType::FilterSquare |
            InstructionType::CtorStruct(_) |
            InstructionType::CtorEnum(_,_) |
            InstructionType::SValue(_,_) |
            InstructionType::RefSValue(_,_) |
            InstructionType::EValue(_,_) |
            InstructionType::RefEValue(_,_) |
            InstructionType::FilterEValue(_,_) |
            InstructionType::ETest(_,_) |
            InstructionType::Proc(_,_) |
            InstructionType::Operator(_) =>
                panic!("Unexpected instruction {:?}",self),

            InstructionType::At |
            InstructionType::Nil |
            InstructionType::Run |
            InstructionType::Add |
            InstructionType::Copy |
            InstructionType::Append |
            InstructionType::Filter |
            InstructionType::ReFilter |
            InstructionType::NumEq |
            InstructionType::Length |
            InstructionType::SeqFilter |
            InstructionType::SeqAt |
            InstructionType::Const(_) |
            InstructionType::NumberConst(_) |
            InstructionType::BooleanConst(_) |
            InstructionType::StringConst(_) |
            InstructionType::BytesConst(_) => 
                vec![0],

            InstructionType::Call(_,_,sigs) => {
                let mut out = Vec::new();
                let mut reg_offset = 0;
                for sig in sigs.iter() {
                    let num_regs = sig.iter().map(|x| x.1.register_count()).sum();
                    if sig.justifies_call() {
                        for i in 0..num_regs {
                            out.push(reg_offset+i);
                        }
                    }
                    reg_offset += num_regs;
                }
                out
            },
        }
    }

    pub fn get_constraints(&self, defstore: &DefStore) -> Result<Vec<ArgumentConstraint>,String> {
        match self {
            InstructionType::CtorStruct(name) => {
                let exprdecl = defstore.get_struct(name).ok_or_else(|| format!("No such struct {:?}",name))?;
                let intypes = exprdecl.get_member_types();
                let mut out = vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::StructType(name.to_string())))];
                out.extend(intypes.iter().map(|t| ArgumentConstraint::NonReference(t.to_argumentexpressionconstraint())));
                Ok(out)
            },

            InstructionType::CtorEnum(name,branch) => {
                let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
                let intype = exprdecl.get_branch_type(branch).ok_or_else(|| format!("No such enum branch {:?}",name))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(name.to_string()))),
                    ArgumentConstraint::NonReference(intype.to_argumentexpressionconstraint())
                ])
            },

            InstructionType::SValue(stype,field) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::StructType(stype.to_string())))
                ])
            },

            InstructionType::RefSValue(stype,field) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::Reference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::Reference(ArgumentExpressionConstraint::Base(BaseType::StructType(stype.to_string())))
                ])
            },

            InstructionType::EValue(etype,field) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(etype.to_string())))
                ])
            },

            InstructionType::FilterEValue(etype,field) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(etype.to_string())))
                ])
            },

            InstructionType::RefEValue(etype,field) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                Ok(vec![
                    ArgumentConstraint::Reference(dtype.to_argumentexpressionconstraint()),
                    ArgumentConstraint::Reference(ArgumentExpressionConstraint::Base(BaseType::EnumType(etype.to_string())))
                ])
            },

            InstructionType::ETest(etype,_) => {
                Ok(vec![
                    fixed(BaseType::BooleanType),
                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::EnumType(etype.to_string())))
                ])
            },

            InstructionType::Proc(name,modes) => {
                let procdecl = defstore.get_proc(name).ok_or_else(|| format!("No such procedure {:?}",name))?;
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

            InstructionType::Operator(name) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_func(name).ok_or_else(|| format!("No such function {:?}",name))?;
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

            InstructionType::NumEq |
            InstructionType::Length |
            InstructionType::Add |
            InstructionType::SeqFilter |
            InstructionType::SeqAt |
            InstructionType::Call(_,_,_) =>
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

    pub fn get_registers(&self) -> Vec<Register> {
        self.regs.iter().cloned().collect()
    }

    pub fn get_constraint(&self, defstore: &DefStore) -> Result<InstructionConstraint,String> {
        let mut out = Vec::new();
        for (i,c) in self.itype.get_constraints(defstore)?.drain(..).enumerate() {
            out.push((c,self.regs[i]));
        }
        Ok(InstructionConstraint::new(&out))
    }
}
