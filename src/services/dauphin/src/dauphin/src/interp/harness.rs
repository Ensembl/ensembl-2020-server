use std::collections::{ HashMap };
use std::rc::Rc;
use crate::generate::InstructionType;
use crate::generate::GenContext;
use crate::model::Register;
use crate::interp::context::InterpContext;
use crate::interp::command::Command;
use crate::interp::commands::core::{
    NilCommand, NumberConstCommand, ConstCommand, BooleanConstCommand, StringConstCommand, BytesConstCommand, CopyCommand,
    AppendCommand, LengthCommand, AddCommand, AtCommand, NumEqCommand, FilterCommand, RunCommand, SeqFilterCommand,
    SeqAtCommand
};
use crate::interp::commands::library::{
     LenCommand, EqCommand, InterpBinBoolCommand, InterpBinBoolOp, PrintVecCommand,
};
use crate::interp::commands::assign::AssignCommand;
use crate::interp::{ InterpValue, StreamContents };

fn stream_strings(stream: &[StreamContents]) -> Vec<String> {
    let mut out = vec![];
    for s in stream {
        match s {
            StreamContents::String(s) => out.push(s.to_string()),
            _ => {}
        }
    }
    out
}

fn export_indexes(ic: &mut InterpContext) -> Result<HashMap<Register,Vec<usize>>,String> {
    let mut out = HashMap::new();
    for (r,iv) in ic.registers().export()?.iter() {
        let iv = Rc::new(iv.copy());
        let v = InterpValue::to_rc_indexes(&iv)?.0;
        out.insert(*r,v.to_vec());
    }
    Ok(out)
}

pub fn mini_interp(context: &GenContext) -> Result<(Vec<Vec<Vec<usize>>>,HashMap<Register,Vec<usize>>,Vec<String>),String> {
    let mut printed = Vec::new();
    let mut ic = InterpContext::new();
    for instr in &context.get_instructions() {
        print!("{}",ic.registers().dump_many(&instr.get_registers())?);
        print!("{:?}",instr);
        match &instr.itype {
            InstructionType::Nil => { NilCommand(instr.regs[0]).execute(&mut ic)?; },
            InstructionType::NumberConst(n) => { NumberConstCommand(instr.regs[0],*n).execute(&mut ic)?; },
            InstructionType::Const(nn) => { ConstCommand(instr.regs[0],nn.iter().map(|x| *x as usize).collect()).execute(&mut ic)?; },
            InstructionType::BooleanConst(n) => { BooleanConstCommand(instr.regs[0],*n).execute(&mut ic)?; },
            InstructionType::StringConst(n) => { StringConstCommand(instr.regs[0],n.to_string()).execute(&mut ic)?; },
            InstructionType::BytesConst(n) => { BytesConstCommand(instr.regs[0],n.to_vec()).execute(&mut ic)?; },
            InstructionType::Copy => { CopyCommand(instr.regs[0],instr.regs[1]).execute(&mut ic)?; },
            InstructionType::Append => { AppendCommand(instr.regs[0],instr.regs[1]).execute(&mut ic)?; },
            InstructionType::Length => { LengthCommand(instr.regs[0],instr.regs[1]).execute(&mut ic)?; },
            InstructionType::Add => { AddCommand(instr.regs[0],instr.regs[1]).execute(&mut ic)?; },
            InstructionType::At => { AtCommand(instr.regs[0],instr.regs[1]).execute(&mut ic)?; },
            InstructionType::NumEq => { NumEqCommand(instr.regs[0],instr.regs[1],instr.regs[2]).execute(&mut ic)?; },
            InstructionType::Filter => { FilterCommand(instr.regs[0],instr.regs[1],instr.regs[2]).execute(&mut ic)?; },
            InstructionType::Run => { RunCommand(instr.regs[0],instr.regs[1],instr.regs[2]).execute(&mut ic)?; },
            InstructionType::SeqFilter => { SeqFilterCommand(instr.regs[0],instr.regs[1],instr.regs[2],instr.regs[3]).execute(&mut ic)?; },
            InstructionType::SeqAt => { SeqAtCommand(instr.regs[0],instr.regs[1]).execute(&mut ic)?; },
            InstructionType::Call(name,_,types) => {
                match &name[..] {
                    "assign" => { AssignCommand(types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "print_regs" => {
                        let mut print = Vec::new();
                        for r in &instr.regs {
                            let d = ic.registers().get(r).borrow().get_shared()?.to_rc_indexes()?.0.to_vec();
                            print.push(d);
                        }
                        printed.push(print);
                    },
                    "print_vec" => { print!("purposes/B{:?}\n",types); PrintVecCommand(types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "len" => { LenCommand(types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "eq" => { EqCommand(types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "lt" => { InterpBinBoolCommand(InterpBinBoolOp::Lt,types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "gt" => { InterpBinBoolCommand(InterpBinBoolOp::Gt,types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "lteq" => { InterpBinBoolCommand(InterpBinBoolOp::LtEq,types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    "gteq" => { InterpBinBoolCommand(InterpBinBoolOp::GtEq,types.to_vec(),instr.regs.to_vec()).execute(&mut ic)?; },
                    _ => { panic!("Bad mini-interp instruction {:?}",instr); }        
                }
            },

            InstructionType::Alias |
            InstructionType::Proc(_,_) |
            InstructionType::Operator(_) |
            InstructionType::CtorStruct(_) |
            InstructionType::CtorEnum(_,_) |
            InstructionType::SValue(_,_) |
            InstructionType::EValue(_,_) |
            InstructionType::ETest(_,_) |
            InstructionType::List |
            InstructionType::Square |
            InstructionType::RefSquare |
            InstructionType::FilterSquare |
            InstructionType::Star =>
                panic!("Illegal instruction")
        }
        ic.registers().commit();
        print!("{}",ic.registers().dump_many(&instr.get_registers())?);
    }
    Ok((printed,export_indexes(&mut ic)?,stream_strings(&ic.stream_take())))
}
