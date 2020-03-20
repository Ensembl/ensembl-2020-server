use crate::generate::{ Instruction };
use crate::typeinf::MemberMode;
use super::codegen::GenContext;

pub fn call(context: &mut GenContext) -> Result<(),String> {
    let mut out = Vec::new();
    for instr in &context.instrs.to_vec() {
        match instr {
            Instruction::Proc(name,regs) => {
                let mut sig = Vec::new();
                for reg in regs {
                    let type_ = context.types.get(&reg.1).unwrap().clone();
                    sig.push((reg.0,type_));
                }
                out.push(Instruction::Call(name.to_string(),sig,regs.iter().map(|x| x.1).collect()));
            },
            Instruction::Operator(name,dst,src) => {
                let mut regs = dst.to_vec();
                regs.append(&mut src.to_vec());
                let types = regs.iter().map(|reg| (MemberMode::RValue,context.types.get(reg).unwrap().clone())).collect();
                out.push(Instruction::Call(name.to_string(),types,regs.to_vec()));
            },
            _ => { out.push(instr.clone()); }
        }
    }
    context.instrs = out;
    Ok(())
}
