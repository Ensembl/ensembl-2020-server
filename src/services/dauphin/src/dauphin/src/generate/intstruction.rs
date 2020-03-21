use std::fmt;

use crate::model::{ DefStore, Register };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, InstructionConstraint, MemberType, MemberMode };

#[derive(Clone,PartialEq)]
pub enum InstructionType {
    Nil(),
    Alias(),
    Copy(),
    List(),
    Append(),
    Square(),
    FilterSquare(),
    RefSquare(),
    Star(),
    At(),
    Filter(),
    Run(),
    NumEq(),
    Length(),
    Add(),
    SeqFilter(),
    SeqAt()
}

impl InstructionType {
    pub fn get_name(&self) -> String {
        match self {
            InstructionType::Nil() => "nil",
            InstructionType::Alias() => "alias",
            InstructionType::Copy() => "copy",
            InstructionType::List() => "list",
            InstructionType::Append() => "append",
            InstructionType::Square() => "square",
            InstructionType::FilterSquare() => "filtersquare",
            InstructionType::RefSquare() => "refsquare",
            InstructionType::Star() => "star",
            InstructionType::At() => "at",
            InstructionType::Filter() => "filter",
            InstructionType::Run() => "run",
            InstructionType::NumEq() => "numeq",
            InstructionType::Length() => "length",
            InstructionType::Add() => "add",
            InstructionType::SeqFilter() => "seqfilter",
            InstructionType::SeqAt() => "seqat",
        }.to_string()
    }

    pub fn get_constraints(&self) -> Vec<ArgumentConstraint> {
        match self {
            InstructionType::Nil()   => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))],
            InstructionType::Alias() => vec![ArgumentConstraint::Reference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                             ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))],
            InstructionType::Copy() =>  vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                             ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))],
            InstructionType::List() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Vec(
                                                Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))],
            InstructionType::Append() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                              ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))],
            InstructionType::Square() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                              ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Vec(
                                                  Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))],
            InstructionType::RefSquare() => vec![ArgumentConstraint::Reference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                                 ArgumentConstraint::Reference(ArgumentExpressionConstraint::Vec(
                                                    Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))],
            InstructionType::FilterSquare() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                                                    ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Vec(
                                                        Box::new(ArgumentExpressionConstraint::Placeholder(String::new()))))],
            InstructionType::Star() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Vec(
                                                Box::new(ArgumentExpressionConstraint::Placeholder(String::new())))),
                                            ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))],
            InstructionType::At() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                                          ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new()))],
            InstructionType::Filter() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                              ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Placeholder(String::new())),
                                              ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::BooleanType))],
            InstructionType::Run() => vec![ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                                           ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType)),
                                           ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType))],
            InstructionType::NumEq() |
            InstructionType::Length() |
            InstructionType::Add() |
            InstructionType::SeqFilter() |
            InstructionType::SeqAt() =>
                vec![],
        }
    }
}

#[derive(Clone,PartialEq)]
pub enum Instruction {
    New(InstructionType,Vec<String>,Vec<Register>),

    /* structs/enums: created at codegeneration, removed at simplification */
    CtorStruct(String,Register,Vec<Register>),
    CtorEnum(String,String,Register,Register),
    SValue(String,String,Register,Register),
    EValue(String,String,Register,Register),
    ETest(String,String,Register,Register),

    /* constant building */
    NumberConst(Register,f64),
    BooleanConst(Register,bool),
    StringConst(Register,String),
    BytesConst(Register,Vec<u8>),

    /* calls-out */
    Proc(String,Vec<(MemberMode,Register)>),
    Operator(String,Vec<Register>,Vec<Register>),
    Call(String,Vec<(MemberMode,MemberType)>,Vec<Register>),
}

fn fmt_instr(f: &mut fmt::Formatter<'_>,opcode: &str, regs: &Vec<&Register>, more: &Vec<String>) -> fmt::Result {
    let mut regs : Vec<String> = regs.iter().map(|x| format!("{:?}",x)).collect();
    if more.len() > 0 { regs.push("".to_string()); }
    write!(f,"#{} {}{};\n",opcode,regs.join(" "),more.join(" "))?;
    Ok(())
}

fn fmt_instr2(f: &mut fmt::Formatter<'_>, opcode: &str, regs: &Vec<Register>, more: &Vec<String>) -> fmt::Result {
    let mut regs : Vec<String> = regs.iter().map(|x| format!("{:?}",x)).collect();
    if more.len() > 0 { regs.push("".to_string()); }
    write!(f,"#{} {}{};\n",opcode,regs.join(" "),more.join(" "))?;
    Ok(())
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::New(itype,prefixes,regs) => {
                let mut name = vec![itype.get_name()];
                name.extend(prefixes.iter().cloned());
                fmt_instr2(f,&name.join(":"),regs,&vec![])?
            },
            Instruction::NumberConst(r0,num) =>
                fmt_instr(f,"number",&vec![r0],&vec![num.to_string()])?,
            Instruction::BooleanConst(r0,b) => 
                fmt_instr(f,"bool",&vec![r0],&vec![b.to_string()])?,
            Instruction::StringConst(r0,s) =>
                fmt_instr(f,"string",&vec![r0],&vec![format!("\"{}\"",s.to_string())])?,
            Instruction::BytesConst(r0,b) => 
                fmt_instr(f,"bytes",&vec![r0],&vec![format!("\'{}\'",hex::encode(b))])?,
            Instruction::Proc(name,regs) =>  {
                let regs : Vec<String> = regs.iter().map(|x| format!("{:?}/{}",x.1,x.0)).collect();
                write!(f,"#proc:{} {};\n",name,regs.join(" "))?;
            },
            Instruction::Operator(name,dsts,srcs) =>  {
                let mut args = Vec::new();
                args.extend(dsts.iter());
                args.extend(srcs.iter());
                fmt_instr(f,&format!("oper:{}",name),&args,&vec![])?
            },
            Instruction::Call(name,types,regs) => {
                let types : Vec<String> = types.iter().map(|x| format!("{:?}/{}",x.1,x.0)).collect();
                fmt_instr(f,&format!("call:{}:{}",name,types.join(":")),&regs.iter().map(|x| x).collect(),&vec![])?;
            },
            Instruction::CtorStruct(name,dest,regs) => {
                let mut r = vec![dest];
                r.extend(regs.iter());
                fmt_instr(f,&format!("struct:{}",name),&r,&vec![])?
            },
            Instruction::CtorEnum(name,branch,dst,src) => {
                fmt_instr(f,&format!("enum:{}:{}",name,branch),&vec![dst,src],&vec![])?
            },
            Instruction::SValue(field,name,dst,src) => {
                fmt_instr(f,&format!("svalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction::EValue(field,name,dst,src) => {
                fmt_instr(f,&format!("evalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction::ETest(field,name,dst,src) => {
                fmt_instr(f,&format!("etest:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
        }
        Ok(())
    }
}

impl Instruction {
    pub fn get_registers(&self) -> Vec<Register> {
        match self {
            Instruction::CtorStruct(_,a,bb) => { let mut out = bb.to_vec(); out.push(a.clone()); out },
            Instruction::CtorEnum(_,_,a,b) => vec![a.clone(),b.clone()],
            Instruction::SValue(_,_,a,b) => vec![a.clone(),b.clone()],
            Instruction::EValue(_,_,a,b) => vec![a.clone(),b.clone()],
            Instruction::ETest(_,_,a,b) => vec![a.clone(),b.clone()],
            Instruction::NumberConst(a,_) => vec![a.clone()],
            Instruction::BooleanConst(a,_) => vec![a.clone()],
            Instruction::StringConst(a,_) => vec![a.clone()],
            Instruction::BytesConst(a,_) => vec![a.clone()],
            Instruction::Proc(_,aa) => aa.iter().map(|x| x.1).collect(),
            Instruction::Operator(_,aa,bb) => { let mut out = aa.to_vec(); out.extend(bb.to_vec()); out },
            Instruction::Call(_,_,aa) => aa.to_vec(),
            Instruction::New(_,_,r) => r.iter().cloned().collect(),
        }
    }

    pub fn get_constraint(&self, defstore: &DefStore) -> Result<InstructionConstraint,String> {
        Ok(InstructionConstraint::new(&match self {
            Instruction::Proc(name,regs) => {
                let procdecl = defstore.get_proc(name).ok_or_else(|| format!("No such procedure {:?}",name))?;
                let signature = procdecl.get_signature();
                let mut arguments = Vec::new();
                let mut member_index = 0;
                let members : Vec<_> = signature.each_member().collect();
                for reg in regs {
                    let constraint = match reg.0 {
                        MemberMode::RValue | MemberMode::LValue => {
                            member_index += 1;
                            members[member_index-1].to_argumentconstraint()
                        },
                        MemberMode::FValue => ArgumentConstraint::NonReference(ArgumentExpressionConstraint::Base(BaseType::NumberType))
                    };
                    arguments.push((constraint,reg.1));
                }
                print!("args: {:?}\n",arguments);
                arguments
            },
            Instruction::CtorStruct(name,dst,srcs) => {
                let mut out = Vec::new();
                out.push((ArgumentConstraint::NonReference(
                    ArgumentExpressionConstraint::Base(
                        BaseType::StructType(name.to_string())
                    )
                ),dst.clone()));
                let exprdecl = defstore.get_struct(name).ok_or_else(|| format!("No such struct {:?}",name))?;
                let intypes = exprdecl.get_member_types();
                if intypes.len() != srcs.len() {
                    return Err(format!("Incorrect number of arguments: got {} expected {}",srcs.len(),intypes.len()));
                }
                for (i,intype) in intypes.iter().enumerate() {
                    out.push((ArgumentConstraint::NonReference(
                        intype.to_argumentexpressionconstraint()
                    ),srcs[i].clone()));
                }
                out
            },
            Instruction::CtorEnum(name,branch,dst,src) => {
                let mut out = Vec::new();
                out.push((ArgumentConstraint::NonReference(
                    ArgumentExpressionConstraint::Base(
                        BaseType::EnumType(name.to_string())
                    )
                ),dst.clone()));
                let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
                out.push((ArgumentConstraint::NonReference(
                    exprdecl.get_branch_type(branch).ok_or_else(|| format!("No such enum branch {:?}",name))?
                        .to_argumentexpressionconstraint()
                ),src.clone()));
                out
            },
            Instruction::SValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                vec![
                    (ArgumentConstraint::NonReference(
                        dtype.to_argumentexpressionconstraint()
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StructType(stype.to_string())
                        )
                    ),src.clone())
                ]
            },
            Instruction::EValue(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                vec![
                    (ArgumentConstraint::NonReference(
                        dtype.to_argumentexpressionconstraint()
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::EnumType(etype.to_string())
                        )
                    ),src.clone())
                ]
            },
            Instruction::ETest(_,etype,dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::EnumType(etype.to_string())
                        )
                    ),src.clone())
                ]
            },
            Instruction::Operator(name,dsts,srcs) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_func(name).ok_or_else(|| format!("No such function {:?}",name))?;
                let signature = exprdecl.get_signature();
                let mut regs = dsts.clone();
                regs.extend(srcs.iter().cloned());
                for (i,member_constraint) in signature.each_member().enumerate() {
                    out.push((
                        member_constraint.to_argumentconstraint()
                    ,regs[i].clone()));
                }
                print!("operator {:?} ({:?})\n",out,signature);
                out
            },
            Instruction::NumberConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),r.clone())
                ]
            },
            Instruction::BooleanConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),r.clone())
                ]
            },
            Instruction::StringConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StringType
                        )
                    ),r.clone())
                ]
            },
            Instruction::BytesConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BytesType
                        )
                    ),r.clone())
                ]
            },
            Instruction::New(itype,_prefixes,regs) => {
                let mut out = Vec::new();
                for (i,c) in itype.get_constraints().drain(..).enumerate() {
                    out.push((c,regs[i]));
                }
                out
            },
            other => return Err(format!("Cannot deduce type of {:?} instructions",other))
        }))
    }
}