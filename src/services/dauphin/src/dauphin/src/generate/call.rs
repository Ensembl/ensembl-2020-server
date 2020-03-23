use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::MemberMode;
use super::codegen::GenContext;

pub fn call(context: &mut GenContext) -> Result<(),String> {
    let mut out = Vec::new();
    for instr in &context.instrs.to_vec() {
        match &instr.itype {
            InstructionType::Proc(name,modes) => {
                let mut sig = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.types.get(&reg).unwrap().clone();
                    sig.push((modes[i],type_));
                }
                out.push(Instruction::new(InstructionType::Call(name.to_string(),sig),instr.regs.to_vec()));
            },

            InstructionType::Operator(name) => {
                let types = instr.regs.iter().map(|reg| (MemberMode::RValue,context.types.get(reg).unwrap().clone())).collect();
                out.push(Instruction::new(InstructionType::Call(name.to_string(),types),instr.regs.to_vec()));
            },

            _ => { out.push(instr.clone()); }
        }
    }
    context.instrs = out;
    Ok(())
}
