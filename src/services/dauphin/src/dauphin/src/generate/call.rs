use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::gencontext::GenContext;
use crate::model::{ RegisterSignature, ComplexRegisters };

pub fn call(context: &mut GenContext) -> Result<(),String> {
    for instr in &context.get_instructions() {
        match &instr.itype {
            InstructionType::Proc(name,modes) => {
                let mut rs = RegisterSignature::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    let flow = match modes[i] {
                        MemberMode::LValue => MemberDataFlow::JustifiesCall,
                        _ => MemberDataFlow::Normal
                    };
                    rs.add(ComplexRegisters::new(&context.get_defstore(),modes[i],&type_,flow)?);
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),true,rs),instr.regs.to_vec()));
            },
            
            InstructionType::Operator(name) => {
                let mut rs = RegisterSignature::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let mode = if i == 0 { MemberDataFlow::JustifiesCall } else { MemberDataFlow::Normal };
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    rs.add(ComplexRegisters::new(&context.get_defstore(),MemberMode::RValue,&type_,mode)?);
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),false,rs),instr.regs.to_vec()));
            },

            _ => { context.add(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}
