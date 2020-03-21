use std::collections::HashMap;
use std::rc::Rc;
use std::fmt;

use crate::model::{ DefStore, Register };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, InstructionConstraint, MemberType, MemberMode };
use super::codegen::GenContext;

pub trait Instruction2 {
    fn get_registers(&self) -> Vec<Register>;
    fn format(&self) -> String;
    fn get_constraint(&self, defstore: &DefStore) -> Result<Vec<(ArgumentConstraint,Register)>,String>;
    fn simplify(&self, defstore: &DefStore, context: &mut GenContext, obj_name: &str,mapping: &HashMap<Register,Vec<Register>>, branch_names: &Vec<String>) -> Result<Vec<Instruction>,String>;
}

#[derive(PartialEq,Clone)]
pub struct Instruction2Core<T> {
    pub prefixes: Vec<String>,
    pub registers: Vec<Register>,
    pub suffixes: Vec<T>
}

impl<T> Instruction2Core<T> {
    pub fn new(prefixes: Vec<String>, registers: Vec<Register>, suffixes: Vec<T>) -> Instruction2Core<T> {
        Instruction2Core { prefixes, registers, suffixes }
    }

    pub fn get_registers(&self) -> Vec<Register> {
        self.registers.to_vec()
    }

    pub fn format<U>(&self, suffix_map: U) -> String where U: Fn(&T) -> Option<String> {
        let mut args = vec![self.prefixes.join(":")];
        args.extend(self.registers.iter().map(|x| format!("{:?}",x)));
        args.extend(self.suffixes.iter().filter_map(|x| suffix_map(x)));
        format!("#{};\n",args.join(" "))
    }
}


#[derive(Clone)]
pub enum Instruction {
    /* structs/enums: created at codegeneration, removed at simplification */
    New(Rc<dyn Instruction2>),

    /* constant building */
    NumberConst(Register,f64),
    BooleanConst(Register,bool),
    StringConst(Register,String),
    BytesConst(Register,Vec<u8>),
    List(Register),
    Append(Register,Register),

    /* housekeeping */
    Copy(Register,Register),
    Alias(Register,Register),
    Nil(Register),

    /* calls-out */
    Proc(String,Vec<(MemberMode,Register)>),
    Operator(String,Vec<Register>,Vec<Register>),
    Call(String,Vec<(MemberMode,MemberType)>,Vec<Register>),

    /* filtering */
    Square(Register,Register),
    FilterSquare(Register,Register),
    RefSquare(Register,Register),
    Star(Register,Register),
    Filter(Register,Register,Register),
    At(Register,Register),
    Run(Register,Register,Register),

    /* opers that are promoted to here because used internally */
    /* introduced in simplify */
    NumEq(Register,Register,Register),
    /* introduced in linearize */
    Length(Register,Register),
    Add(Register,Register),
    SeqFilter(Register,Register,Register,Register),
    SeqAt(Register,Register)
}

fn fmt_instr(f: &mut fmt::Formatter<'_>,opcode: &str, regs: &Vec<&Register>, more: &Vec<String>) -> fmt::Result {
    let mut regs : Vec<String> = regs.iter().map(|x| format!("{:?}",x)).collect();
    if more.len() > 0 { regs.push("".to_string()); }
    write!(f,"#{} {}{};\n",opcode,regs.join(" "),more.join(" "))?;
    Ok(())
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::New(n) => {
                write!(f,"{}",n.format())?
            },
            Instruction::Nil(r) =>
                fmt_instr(f,"nil",&vec![r],&vec![])?,
            Instruction::NumberConst(r0,num) =>
                fmt_instr(f,"number",&vec![r0],&vec![num.to_string()])?,
            Instruction::BooleanConst(r0,b) => 
                fmt_instr(f,"bool",&vec![r0],&vec![b.to_string()])?,
            Instruction::StringConst(r0,s) =>
                fmt_instr(f,"string",&vec![r0],&vec![format!("\"{}\"",s.to_string())])?,
            Instruction::BytesConst(r0,b) => 
                fmt_instr(f,"bytes",&vec![r0],&vec![format!("\'{}\'",hex::encode(b))])?,
            Instruction::List(r0) =>
                fmt_instr(f,"list",&vec![r0],&vec![])?,
            Instruction::Length(r0,r1) =>
                fmt_instr(f,"length",&vec![r0,r1],&vec![])?,
            Instruction::Append(r0,r1) => 
                fmt_instr(f,"append",&vec![r0,r1],&vec![])?,
            Instruction::Add(r0,r1) => 
                fmt_instr(f,"add",&vec![r0,r1],&vec![])?,
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
            Instruction::Square(dst,src) => {
                fmt_instr(f,"square",&vec![dst,src],&vec![])?
            },
            Instruction::RefSquare(dst,src) => {
                fmt_instr(f,"refsquare",&vec![dst,src],&vec![])?
            },
            Instruction::FilterSquare(dst,src) => {
                fmt_instr(f,"filtersquare",&vec![dst,src],&vec![])?
            },
            Instruction::Star(dst,src) => {
                fmt_instr(f,"star",&vec![dst,src],&vec![])?
            },
            Instruction::Alias(dst,src) => {
                fmt_instr(f,"alias",&vec![dst,src],&vec![])?
            },
            Instruction::At(dst,src) => {
                fmt_instr(f,"at",&vec![dst,src],&vec![])?
            },
            Instruction::Run(dst,offset,len) => {
                fmt_instr(f,"run",&vec![dst,offset,len],&vec![])?
            },
            Instruction::SeqAt(dst,src) => {
                fmt_instr(f,"seqat",&vec![dst,src],&vec![])?
            },
            Instruction::Filter(dst,src,filter) => {
                fmt_instr(f,"filter",&vec![dst,src,filter],&vec![])?
            },
            Instruction::SeqFilter(dst,src,start,len) => {
                fmt_instr(f,"seqfilter",&vec![dst,src,start,len],&vec![])?
            },
            Instruction::Copy(dst,src) => {
                fmt_instr(f,"copy",&vec![dst,src],&vec![])?
            },
            Instruction::NumEq(out,a,b) => {
                fmt_instr(f,"numeq",&vec![out,a,b],&vec![])?
            },
        }
        Ok(())
    }
}

impl Instruction {
    pub fn get_registers(&self) -> Vec<Register> {
        match self {
            Instruction::New(n) => n.get_registers(),
            Instruction::NumberConst(a,_) => vec![a.clone()],
            Instruction::BooleanConst(a,_) => vec![a.clone()],
            Instruction::StringConst(a,_) => vec![a.clone()],
            Instruction::BytesConst(a,_) => vec![a.clone()],
            Instruction::List(a) => vec![a.clone()],
            Instruction::Append(a,b) => vec![a.clone(),b.clone()],
            Instruction::Add(a,b) => vec![a.clone(),b.clone()],
            Instruction::Copy(a,b) => vec![a.clone(),b.clone()],
            Instruction::Alias(a,b) => vec![a.clone(),b.clone()],
            Instruction::Proc(_,aa) => aa.iter().map(|x| x.1).collect(),
            Instruction::Operator(_,aa,bb) => { let mut out = aa.to_vec(); out.extend(bb.to_vec()); out },
            Instruction::Call(_,_,aa) => aa.to_vec(),
            Instruction::Square(a,b) => vec![a.clone(),b.clone()],
            Instruction::RefSquare(a,b) => vec![a.clone(),b.clone()],
            Instruction::FilterSquare(a,b) => vec![a.clone(),b.clone()],
            Instruction::Star(a,b) => vec![a.clone(),b.clone()],
            Instruction::Filter(a,b,c) => vec![a.clone(),b.clone(),c.clone()],
            Instruction::SeqFilter(a,b,c,d) => vec![a.clone(),b.clone(),c.clone(),d.clone()],
            Instruction::At(a,b) => vec![a.clone(),b.clone()],
            Instruction::Run(a,b,c) => vec![a.clone(),b.clone(),c.clone()],
            Instruction::SeqAt(a,b) => vec![a.clone(),b.clone()],
            Instruction::NumEq(a,b,c) => vec![a.clone(),b.clone(),c.clone()],
            Instruction::Nil(a) => vec![a.clone()],
            Instruction::Length(a,b) => vec![a.clone(),b.clone()],
        }
    }

    pub fn get_constraint(&self, defstore: &DefStore) -> Result<InstructionConstraint,String> {
        Ok(InstructionConstraint::new(&match self {
            Instruction::New(b) => b.get_constraint(defstore)?,
            Instruction::Alias(dst,src) => {
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone())
                ]
            },
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
            Instruction::Copy(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
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
            Instruction::List(r) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),r.clone())
                ]
            },
            Instruction::Nil(r) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),r.clone())
                ]
            },
            Instruction::Append(r,c) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),r.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),c.clone())
                ]
            },
            Instruction::Star(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone())
                ]
            },
            Instruction::Square(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),src.clone())
                ]
            },
            Instruction::RefSquare(dst,src) => {
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),src.clone())
                ]
            },
            Instruction::FilterSquare(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),src.clone())
                ]
            },
            Instruction::Filter(dst,src,filter) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),filter.clone()),
                ]
            },
            Instruction::At(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone()),
                ]
            },
            Instruction::Run(dst,offset,len) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),offset.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),len.clone()),
                ]
            },
            other => return Err(format!("Cannot deduce type of {:?} instructions",other))
        }))
    }
}