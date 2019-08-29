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
    RefSValue2(String,String,Register,Register),
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

    Set(Register,Register),

    /* to go */
    Ref(Register,Register),
    RefSValue(String,String,Register,Register),
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
            Instruction::RefSValue2(field,name,dst,src) => {
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
            Instruction::Set(dst,src) => {
                fmt_instr(f,"set",&vec![dst,src],&vec![])?
            }
        }
        Ok(())
    }
}
