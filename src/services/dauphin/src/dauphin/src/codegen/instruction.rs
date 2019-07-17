use std::fmt;

use super::register::Register;

pub enum Instruction {
    NumberConst(Register,f64),
    BooleanConst(Register,bool),
    StringConst(Register,String),
    BytesConst(Register,Vec<u8>),
    CtorStruct(String,Register,Vec<Register>),
    CtorEnum(String,String,Register,Register),
    List(Register),
    Push(Register,Register),
    Proc(String,Vec<Register>),
    Dot(String,Register,Register), // becomes #svalue after type inference
    Query(String,Register,Register), // becomes #etest after type inference
    Pling(String,Register,Register), // becomes #evalue after type inference
    RefDot(String,Register,Register), // becomes #refsvalue after type inference
    RefPling(String,Register,Register), // becomes #refevalue after type inference
    Square(Register,Register),
    RefSquare(Register,Register),
    Star(Register,Register),
    Filter(Register,Register,Register),
    RefFilter(Register,Register,Register),
    At(Register,Register),
    Operator(String,Vec<Register>),
    Ref(Register,Register),
    RefStar(Register,Register),
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
            Instruction::Operator(name,regs) => 
                fmt_instr(f,&format!("oper:{}",name),&regs.iter().map(|x| x).collect(),&vec![])?,
            Instruction::CtorStruct(name,dest,regs) => {
                let mut r = vec![dest];
                r.extend(regs.iter());
                fmt_instr(f,&format!("struct:{}",name),&r,&vec![])?
            },
            Instruction::CtorEnum(name,branch,dst,src) => {
                fmt_instr(f,&format!("enum:{}:{}",name,branch),&vec![dst,src],&vec![])?
            },
            Instruction::Dot(field,dst,src) => {
                fmt_instr(f,&format!("dot:{}",field),&vec![dst,src],&vec![])?
            },
            Instruction::RefDot(field,dst,src) => {
                fmt_instr(f,&format!("refdot:{}",field),&vec![dst,src],&vec![])?
            },
            Instruction::Query(field,dst,src) => {
                fmt_instr(f,&format!("query:{}",field),&vec![dst,src],&vec![])?
            },
            Instruction::Pling(field,dst,src) => {
                fmt_instr(f,&format!("pling:{}",field),&vec![dst,src],&vec![])?
            },
            Instruction::RefPling(field,dst,src) => {
                fmt_instr(f,&format!("refpling:{}",field),&vec![dst,src],&vec![])?
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
            Instruction::RefStar(dst,src) => {
                fmt_instr(f,"refstar",&vec![dst,src],&vec![])?
            }
        }
        Ok(())
    }
}
