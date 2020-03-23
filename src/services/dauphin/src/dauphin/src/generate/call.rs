use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::MemberMode;
use super::gencontext::GenContext;

pub fn call(context: &mut GenContext) -> Result<(),String> {
    for instr in &context.get_instructions() {
        match &instr.itype {
            InstructionType::Proc(name,modes) => {
                let mut sig = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    sig.push((modes[i],type_));
                }
                context.add_instruction(Instruction::new(InstructionType::Call(name.to_string(),sig),instr.regs.to_vec()));
            },

            InstructionType::Operator(name) => {
                let types = instr.regs.iter().map(|reg| (MemberMode::RValue,context.xxx_types().get(reg).unwrap().clone())).collect();
                context.add_instruction(Instruction::new(InstructionType::Call(name.to_string(),types),instr.regs.to_vec()));
            },

            _ => { context.add_instruction(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}
