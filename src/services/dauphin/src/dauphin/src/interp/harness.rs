use std::collections::{ HashMap };
use std::rc::Rc;
use crate::generate::{ Instruction, InstructionType };
use crate::generate::GenContext;
use crate::model::Register;
use crate::interp::context::InterpContext;
use crate::interp::command::Command;
use crate::interp::commands::core::core::{
    CopyCommand, AppendCommand, LengthCommand, AddCommand, NumEqCommand, FilterCommand, RunCommand,
    SeqFilterCommand, SeqAtCommand, NilCommand, AtCommand, ReFilterCommand
};
use crate::interp::commands::core::consts::{
    NumberConstCommand, ConstCommand, BooleanConstCommand, StringConstCommand, BytesConstCommand
};
use crate::typeinf::MemberMode;

use crate::interp::commands::library::{
     LenCommand, EqCommand, InterpBinBoolCommand, InterpBinBoolOp, PrintVecCommand, PrintRegsCommand,
     AssertCommand
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
        let v = InterpValue::to_rc_indexes(&iv).map(|x| x.0.to_vec()).unwrap_or(vec![]);
        out.insert(*r,v);
    }
    Ok(out)
}

fn instruction_to_command(instr: &Instruction) -> Box<dyn Command> {
    match &instr.itype {
        InstructionType::Nil => { Box::new(NilCommand(instr.regs[0])) },
        InstructionType::NumberConst(n) => { Box::new(NumberConstCommand(instr.regs[0],*n)) },
        InstructionType::Const(nn) => { Box::new(ConstCommand(instr.regs[0],nn.iter().map(|x| *x as usize).collect())) },
        InstructionType::BooleanConst(n) => { Box::new(BooleanConstCommand(instr.regs[0],*n)) },
        InstructionType::StringConst(n) => { Box::new(StringConstCommand(instr.regs[0],n.to_string())) },
        InstructionType::BytesConst(n) => { Box::new(BytesConstCommand(instr.regs[0],n.to_vec())) },
        InstructionType::Copy => { Box::new(CopyCommand(instr.regs[0],instr.regs[1])) },
        InstructionType::Append => { Box::new(AppendCommand(instr.regs[0],instr.regs[1])) },
        InstructionType::Length => { Box::new(LengthCommand(instr.regs[0],instr.regs[1])) },
        InstructionType::Add => { Box::new(AddCommand(instr.regs[0],instr.regs[1])) },
        InstructionType::NumEq => { Box::new(NumEqCommand(instr.regs[0],instr.regs[1],instr.regs[2])) },
        InstructionType::ReFilter => { Box::new(ReFilterCommand(instr.regs[0],instr.regs[1],instr.regs[2])) },
        InstructionType::Filter => { Box::new(FilterCommand(instr.regs[0],instr.regs[1],instr.regs[2])) },
        InstructionType::Run => { Box::new(RunCommand(instr.regs[0],instr.regs[1],instr.regs[2])) },
        InstructionType::SeqFilter => { Box::new(SeqFilterCommand(instr.regs[0],instr.regs[1],instr.regs[2],instr.regs[3])) },
        InstructionType::SeqAt => { Box::new(SeqAtCommand(instr.regs[0],instr.regs[1])) },
        InstructionType::At => { Box::new(AtCommand(instr.regs[0],instr.regs[1])) },
        InstructionType::Call(name,_,types) => {
            match &name[..] {
                "assign" => { Box::new(AssignCommand(types[0].0 != MemberMode::LValue,types.iter().map(|x| x.1.clone()).collect(),instr.regs.to_vec())) },
                "print_regs" => { Box::new(PrintRegsCommand(instr.regs.to_vec())) },
                "print_vec" => { Box::new(PrintVecCommand(types[0].1.clone(),instr.regs.to_vec())) },
                "len" => { Box::new(LenCommand(types[1].1.clone(),instr.regs.to_vec())) },
                "eq" => { Box::new(EqCommand(instr.regs.to_vec())) },
                "lt" => { Box::new(InterpBinBoolCommand(InterpBinBoolOp::Lt,instr.regs.to_vec())) },
                "gt" => { Box::new(InterpBinBoolCommand(InterpBinBoolOp::Gt,instr.regs.to_vec())) },
                "lteq" => { Box::new(InterpBinBoolCommand(InterpBinBoolOp::LtEq,instr.regs.to_vec())) },
                "gteq" => { Box::new(InterpBinBoolCommand(InterpBinBoolOp::GtEq,instr.regs.to_vec())) },
                "assert" => { Box::new(AssertCommand(instr.regs[0],instr.regs[1])) }
                _ => { panic!("Bad mini-interp instruction {:?}",instr); }        
            }
        },
        InstructionType::Alias |
        InstructionType::Proc(_,_) |
        InstructionType::Operator(_) |
        InstructionType::CtorStruct(_) |
        InstructionType::CtorEnum(_,_) |
        InstructionType::SValue(_,_) |
        InstructionType::RefSValue(_,_) |
        InstructionType::EValue(_,_) |
        InstructionType::RefEValue(_,_) |
        InstructionType::FilterEValue(_,_) |
        InstructionType::ETest(_,_) |
        InstructionType::List |
        InstructionType::Square |
        InstructionType::RefSquare |
        InstructionType::FilterSquare |
        InstructionType::Star =>
            panic!("Illegal instruction")
    }
}

fn instructions_to_commands(instrs: &Vec<Instruction>) -> Vec<Box<dyn Command>> {
    instrs.iter().map(|ins| instruction_to_command(ins)).collect()
}

pub fn mini_interp(context: &GenContext) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    let mut ic = InterpContext::new();
    let instrs = &context.get_instructions();
    let commands = instructions_to_commands(&instrs);
    for (i,command) in commands.iter().enumerate() {
        let instr = &instrs[i];
        print!("{}",ic.registers().dump_many(&instr.get_registers())?);
        print!("{:?}",instr);
        command.execute(&mut ic)?;
        ic.registers().commit();
        print!("{}",ic.registers().dump_many(&instr.get_registers())?);
    }
    Ok((export_indexes(&mut ic)?,stream_strings(&ic.stream_take())))
}
