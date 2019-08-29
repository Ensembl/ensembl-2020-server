use std::fmt;

use super::register::Register;

#[derive(Clone)]
pub enum Instruction {
    /* generate to simplify */
    CtorStruct(String,Register,Vec<Register>),
    CtorEnum(String,String,Register,Register),
    SValue(String,String,Register,Register),
    EValue(String,String,Register,Register),
    ETest(String,String,Register,Register),
    RefSValue(String,String,Register,Register),
    RefEValue(String,String,Register,Register),

    /* simplify to ??? */
    Copy(Register,Register),

    /* ??? */
    NumberConst(Register,f64),
    BooleanConst(Register,bool),
    StringConst(Register,String),
    BytesConst(Register,Vec<u8>),
    List(Register),
    Push(Register,Register),
    Proc(String,Vec<Register>),
    Square(Register,Register),
    RefSquare(Register,Register),
    Star(Register,Register),
    Filter(Register,Register,Register),
    RefFilter(Register,Register,Register),
    At(Register,Register),
    Operator(String,Register,Vec<Register>),
    Ref(Register,Register)
}

fn fmt_instr(f: &mut fmt::Formatter<'_>,opcode: &str, regs: &Vec<&Register>, more: &Vec<String>) -> fmt::Result {
    let mut regs : Vec<String> = regs.iter().map(|x| format!("{:?}",x)).collect();
    if more.len() > 0 { regs.push("".to_string()); }
    write!(f,"#{} {}{};\n",opcode,regs.join(" "),more.join(" "))?;
    Ok(())
}

impl Instruction {
    pub fn replace_regs(&self, new: &Vec<Register>) -> Result<Instruction,String> {
        match self {
            Instruction::Proc(name,_) => Ok(Instruction::Proc(name.to_string(),new.clone())),
            Instruction::NumberConst(_,c) => Ok(Instruction::NumberConst(new[0].clone().clone(),*c)),
            Instruction::BooleanConst(_,c) => Ok(Instruction::BooleanConst(new[0].clone(),*c)),
            Instruction::StringConst(_,c) => Ok(Instruction::StringConst(new[0].clone(),c.clone())),
            Instruction::BytesConst(_,c) => Ok(Instruction::BytesConst(new[0].clone(),c.clone())),
            Instruction::List(_) => Ok(Instruction::List(new[0].clone())),
            Instruction::Star(_,_) => Ok(Instruction::Star(new[0].clone(),new[1].clone())),
            Instruction::Square(_,_) => Ok(Instruction::Square(new[0].clone(),new[1].clone())),
            Instruction::At(_,_) => Ok(Instruction::At(new[0].clone(),new[1].clone())),
            Instruction::Filter(_,_,_) => Ok(Instruction::Filter(new[0].clone(),new[1].clone(),new[2].clone())),
            Instruction::Push(_,_) => Ok(Instruction::Push(new[0].clone(),new[1].clone())),
            Instruction::CtorEnum(name,branch,_,_) => Ok(Instruction::CtorEnum(name.to_string(),branch.to_string(),new[0].clone(),new[1].clone())),
            Instruction::CtorStruct(name,_,_) => Ok(Instruction::CtorStruct(name.to_string(),new[0].clone(),new[1..].to_vec())),
            Instruction::SValue(field,stype,_,_) => Ok(Instruction::SValue(field.to_string(),stype.to_string(),new[0].clone(),new[1].clone())),
            Instruction::EValue(field,etype,_,_) => Ok(Instruction::EValue(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
            Instruction::ETest(field,etype,_,_) => Ok(Instruction::ETest(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
            Instruction::RefSValue(field,stype,_,_) => Ok(Instruction::RefSValue(field.to_string(),stype.to_string(),new[0].clone(),new[1].clone())),
            Instruction::RefEValue(field,etype,_,_) => Ok(Instruction::RefEValue(field.to_string(),etype.to_string(),new[0].clone(),new[1].clone())),
            Instruction::RefSquare(_,_) => Ok(Instruction::RefSquare(new[0].clone(),new[1].clone())),
            Instruction::RefFilter(_,_,_) => Ok(Instruction::RefFilter(new[0].clone(),new[1].clone(),new[2].clone())),
            Instruction::Operator(name,_,_) => Ok(Instruction::Operator(name.to_string(),new[0].clone(),new[1..].to_vec())),
            Instruction::Copy(_,_) => Ok(Instruction::Copy(new[0].clone(),new[1].clone())),
            Instruction::Ref(_,_) => Ok(Instruction::Ref(new[0].clone(),new[1].clone())),
        }
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Instruction::Push(r0,r1) => 
                fmt_instr(f,"push",&vec![r0,r1],&vec![])?,
            Instruction::Proc(name,regs) => 
                fmt_instr(f,&format!("proc:{}",name),&regs.iter().map(|x| x).collect(),&vec![])?,
            Instruction::Operator(name,dst,srcs) =>  {
                let mut r = vec![dst];
                r.extend(srcs.iter());
                fmt_instr(f,&format!("oper:{}",name),&r,&vec![])?
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
            Instruction::RefSValue(field,name,dst,src) => {
                fmt_instr(f,&format!("refsvalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction::RefEValue(field,name,dst,src) => {
                fmt_instr(f,&format!("refevalue:{}:{}",name,field),&vec![dst,src],&vec![])?
            },
            Instruction::Square(dst,src) => {
                fmt_instr(f,"square",&vec![dst,src],&vec![])?
            },
            Instruction::RefSquare(dst,src) => {
                fmt_instr(f,"refsquare",&vec![dst,src],&vec![])?
            },
            Instruction::Star(dst,src) => {
                fmt_instr(f,"star",&vec![dst,src],&vec![])?
            },
            Instruction::Ref(dst,src) => {
                fmt_instr(f,"ref",&vec![dst,src],&vec![])?
            },
            Instruction::At(dst,src) => {
                fmt_instr(f,"at",&vec![dst,src],&vec![])?
            },
            Instruction::Filter(dst,src,filter) => {
                fmt_instr(f,"filter",&vec![dst,src,filter],&vec![])?
            },
            Instruction::RefFilter(dst,src,filter) => {
                fmt_instr(f,"reffilter",&vec![dst,src,filter],&vec![])?
            },
            Instruction::Copy(dst,src) => {
                fmt_instr(f,"copy",&vec![dst,src],&vec![])?
            },
        }
        Ok(())
    }
}
