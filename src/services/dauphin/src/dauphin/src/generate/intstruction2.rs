use std::fmt;

use crate::codegen::{ DefStore, Register2 };
use crate::typeinf::{ ArgumentConstraint, ArgumentExpressionConstraint, BaseType, InstructionConstraint };

pub enum Instruction2 {
    Proc(String,Vec<Register2>),
    NumberConst(Register2,f64),
    BooleanConst(Register2,bool),
    StringConst(Register2,String),
    BytesConst(Register2,Vec<u8>),
    List(Register2),
    Push(Register2,Register2),
    CtorStruct(String,Register2,Vec<Register2>),
    CtorEnum(String,String,Register2,Register2),
    SValue(String,String,Register2,Register2),
    Copy(Register2,Register2),
    EValue(String,String,Register2,Register2),
    ETest(String,String,Register2,Register2),
    RefSValue(String,String,Register2,Register2),
    RefEValue(String,String,Register2,Register2),
    Operator(String,Register2,Vec<Register2>),
    Square(Register2,Register2),
    RefSquare(Register2,Register2),
    Star(Register2,Register2),
    Filter(Register2,Register2,Register2),
    At(Register2,Register2),
    RefFilter(Register2,Register2,Register2),
    Ref(Register2,Register2)
}

fn fmt_instr(f: &mut fmt::Formatter<'_>,opcode: &str, regs: &Vec<&Register2>, more: &Vec<String>) -> fmt::Result {
    let mut regs : Vec<String> = regs.iter().map(|x| format!("{:?}",x)).collect();
    if more.len() > 0 { regs.push("".to_string()); }
    write!(f,"#{} {}{};\n",opcode,regs.join(" "),more.join(" "))?;
    Ok(())
}

impl fmt::Debug for Instruction2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction2::NumberConst(r0,num) =>
                fmt_instr(f,"number",&vec![r0],&vec![num.to_string()])?,
            Instruction2::BooleanConst(r0,b) => 
                fmt_instr(f,"bool",&vec![r0],&vec![b.to_string()])?,
            Instruction2::StringConst(r0,s) =>
                fmt_instr(f,"string",&vec![r0],&vec![format!("\"{}\"",s.to_string())])?,
            Instruction2::BytesConst(r0,b) => 
                fmt_instr(f,"bytes",&vec![r0],&vec![format!("\'{}\'",hex::encode(b))])?,
            Instruction2::List(r0) =>
                fmt_instr(f,"list",&vec![r0],&vec![])?,
            Instruction2::Push(r0,r1) => 
                fmt_instr(f,"push",&vec![r0,r1],&vec![])?,
            Instruction2::Proc(name,regs) => 
                fmt_instr(f,&format!("proc:{}",name),&regs.iter().map(|x| x).collect(),&vec![])?,
            Instruction2::Operator(name,dst,srcs) =>  {
                let mut r = vec![dst];
                r.extend(srcs.iter());
                fmt_instr(f,&format!("oper:{}",name),&r,&vec![])?
            },
            Instruction2::CtorStruct(name,dest,regs) => {
                let mut r = vec![dest];
                r.extend(regs.iter());
                fmt_instr(f,&format!("struct:{}",name),&r,&vec![])?
            },
            Instruction2::CtorEnum(name,branch,dst,src) => {
                fmt_instr(f,&format!("enum:{}:{}",name,branch),&vec![dst,src],&vec![])?
            },
            Instruction2::SValue(field,name,dst,src) => {
                fmt_instr(f,&format!("svalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction2::EValue(field,name,dst,src) => {
                fmt_instr(f,&format!("evalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction2::ETest(field,name,dst,src) => {
                fmt_instr(f,&format!("etest:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction2::RefSValue(field,name,dst,src) => {
                fmt_instr(f,&format!("refsvalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction2::RefEValue(field,name,dst,src) => {
                fmt_instr(f,&format!("refevalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction2::Square(dst,src) => {
                fmt_instr(f,"square",&vec![dst,src],&vec![])?
            },
            Instruction2::RefSquare(dst,src) => {
                fmt_instr(f,"refsquare",&vec![dst,src],&vec![])?
            },
            Instruction2::Star(dst,src) => {
                fmt_instr(f,"star",&vec![dst,src],&vec![])?
            },
            Instruction2::Ref(dst,src) => {
                fmt_instr(f,"ref",&vec![dst,src],&vec![])?
            },
            Instruction2::At(dst,src) => {
                fmt_instr(f,"at",&vec![dst,src],&vec![])?
            },
            Instruction2::Filter(dst,src,filter) => {
                fmt_instr(f,"filter",&vec![dst,src,filter],&vec![])?
            },
            Instruction2::RefFilter(dst,src,filter) => {
                fmt_instr(f,"reffilter",&vec![dst,src,filter],&vec![])?
            },
            Instruction2::Copy(dst,src) => {
                fmt_instr(f,"copy",&vec![dst,src],&vec![])?
            },
        }
        Ok(())
    }
}

impl Instruction2 {
    pub fn get_constraint(&self, defstore: &DefStore) -> Result<InstructionConstraint,String> {
        Ok(InstructionConstraint::new(&match self {
            Instruction2::Ref(dst,src) => {
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone())
                ]
            },
            Instruction2::Proc(name,regs) => {
                let procdecl = defstore.get_proc(name).ok_or_else(|| format!("No such procedure {:?}",name))?;
                let signature = procdecl.get_signature();
                let mut arguments = Vec::new();
                for (i,member) in signature.each_member().enumerate() {
                    arguments.push((member.to_argumentconstraint(),regs[i].clone()));
                }
                arguments
            },
            Instruction2::Copy(dst,src) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone())
                ]
            },
            Instruction2::CtorStruct(name,dst,srcs) => {
                let mut out = Vec::new();
                out.push((ArgumentConstraint::NonReference(
                    ArgumentExpressionConstraint::Base(
                        BaseType::StructType(name.to_string())
                    )
                ),dst.clone()));
                let exprdecl = defstore.get_struct(name).ok_or_else(|| format!("No such struct {:?}",name))?;
                let intypes = exprdecl.get_member_types2();
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
            Instruction2::CtorEnum(name,branch,dst,src) => {
                let mut out = Vec::new();
                out.push((ArgumentConstraint::NonReference(
                    ArgumentExpressionConstraint::Base(
                        BaseType::EnumType(name.to_string())
                    )
                ),dst.clone()));
                let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
                out.push((ArgumentConstraint::NonReference(
                    exprdecl.get_branch_type2(branch).ok_or_else(|| format!("No such enum branch {:?}",name))?
                        .to_argumentexpressionconstraint()
                ),src.clone()));
                out
            },
            Instruction2::SValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type2(field).ok_or_else(|| format!("No such field {:?}",field))?;
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
            Instruction2::RefSValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type2(field).ok_or_else(|| format!("No such field {:?}",field))?;
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StructType(stype.to_string())
                        )
                    ),src.clone()),
                    (ArgumentConstraint::Reference(
                        dtype.to_argumentexpressionconstraint()
                    ),dst.clone())
                ]
            },
            Instruction2::EValue(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type2(field).ok_or_else(|| format!("No such branch {:?}",field))?;
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
            Instruction2::RefEValue(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type2(field).ok_or_else(|| format!("No such field {:?}",field))?;
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::EnumType(etype.to_string())
                        )
                    ),src.clone()),
                    (ArgumentConstraint::Reference(
                        dtype.to_argumentexpressionconstraint()
                    ),dst.clone())
                ]
            },
            Instruction2::ETest(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
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
            Instruction2::Operator(name,dst,srcs) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_func(name).ok_or_else(|| format!("No such function {:?}",name))?;
                let signature = exprdecl.get_signature();
                let mut regs = vec![dst.clone()];
                regs.extend(srcs.iter().cloned());
                for (i,member_constraint) in signature.each_member().enumerate() {
                    out.push((
                        member_constraint.to_argumentconstraint()
                    ,regs[i].clone()));
                }
                print!("operator {:?} ({:?})\n",out,signature);
                out
            },
            Instruction2::NumberConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::NumberType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::BooleanConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::StringConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::StringType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::BytesConst(r,_) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BytesType
                        )
                    ),r.clone())
                ]
            },
            Instruction2::List(r) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),r.clone())
                ]
            },
            Instruction2::Push(r,c) => {
                vec![
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),r.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),c.clone())
                ]
            },
            Instruction2::Star(dst,src) => {
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
            Instruction2::Square(dst,src) => {
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
            Instruction2::RefSquare(dst,src) => {
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Vec(Box::new(
                            ArgumentExpressionConstraint::Placeholder(String::new())
                        ))
                    ),src.clone()),
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone())
                ]
            },
            Instruction2::Filter(dst,src,filter) => {
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
            Instruction2::RefFilter(dst,src,filter) => {
                vec![
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),dst.clone()),
                    (ArgumentConstraint::Reference(
                        ArgumentExpressionConstraint::Placeholder(String::new())
                    ),src.clone()),
                    (ArgumentConstraint::NonReference(
                        ArgumentExpressionConstraint::Base(
                            BaseType::BooleanType
                        )
                    ),filter.clone()),
                ]
            },
            Instruction2::At(dst,src) => {
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
            }
        }))
    }
}