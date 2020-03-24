use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::gencontext::GenContext;

pub fn call(context: &mut GenContext) -> Result<(),String> {
    for instr in &context.get_instructions() {
        match &instr.itype {
            InstructionType::Proc(name,modes) => {
                let mut sig = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    sig.push((modes[i],type_,MemberDataFlow::SelfJustifying));
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),sig),instr.regs.to_vec()));
            },

            InstructionType::Operator(name) => {
                let mut types = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let mode = if i == 0 { MemberDataFlow::JustifiesCall } else { MemberDataFlow::Normal };
                    types.push((MemberMode::RValue,context.xxx_types().get(reg).unwrap().clone(),mode));
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),types),instr.regs.to_vec()));
            },

            _ => { context.add(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}
