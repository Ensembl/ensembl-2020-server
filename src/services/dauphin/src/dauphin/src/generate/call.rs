use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::gencontext::GenContext;
use crate::model::offset;

pub fn call(context: &mut GenContext) -> Result<(),String> {
    for instr in &context.get_instructions() {
        match &instr.itype {
            InstructionType::Proc(name,modes) => {
                let mut sig = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    let flow = match modes[i] {
                        MemberMode::LValue => MemberDataFlow::JustifiesCall,
                        _ => MemberDataFlow::Normal
                    };
                    let purposes = offset(&context.get_defstore(),&type_)?;
                    sig.push((modes[i],purposes,flow));
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),true,sig),instr.regs.to_vec()));
            },
            
            InstructionType::Operator(name) => {
                let mut types = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let mode = if i == 0 { MemberDataFlow::JustifiesCall } else { MemberDataFlow::Normal };
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    let purposes = offset(&context.get_defstore(),&type_)?;
                    types.push((MemberMode::RValue,purposes,mode));
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),false,types),instr.regs.to_vec()));
            },

            _ => { context.add(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}
