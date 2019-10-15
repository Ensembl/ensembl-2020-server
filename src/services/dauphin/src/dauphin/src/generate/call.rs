use crate::generate::{ Instruction };
use crate::typeinf::MemberType;
use super::codegen::GenContext;

pub fn call(context: &mut GenContext) -> Result<(),String> {
    let mut out = Vec::new();
    for instr in &context.instrs.to_vec() {
        match instr {
            Instruction::Proc(name,regs) => {
                let types = regs.iter().map(|reg| context.types.get(reg).unwrap().clone()).collect();
                out.push(Instruction::Call(name.to_string(),types,regs.to_vec()));
            },
            Instruction::Operator(name,dst,src) => {
                let mut regs = dst.to_vec();
                regs.append(&mut src.to_vec());
                let types : Vec<MemberType> = regs.iter().map(|reg| context.types.get(reg).unwrap().clone()).collect();
                out.push(Instruction::Call(name.to_string(),types,regs.to_vec()));
            },
            _ => { out.push(instr.clone()); }
        }
    }
    context.instrs = out;
    Ok(())
}
