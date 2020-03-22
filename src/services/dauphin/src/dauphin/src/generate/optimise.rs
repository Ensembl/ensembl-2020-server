use std::collections::HashSet;
use super::codegen::GenContext;
use super::instruction::Instruction;
use crate::model::Register;

fn add_used_registers(used: &mut HashSet<Register>, instr: &Instruction) {
    for reg in instr.get_registers() {
        used.insert(reg);
    }
}

pub fn remove_unused_registers(context: &mut GenContext) {
    let mut used = HashSet::new();
    for instr in &context.instrs {
        add_used_registers(&mut used,instr);
    }
    let mut unused = Vec::new();
    for (reg,_) in context.types.each_register() {
        if !used.contains(reg) {
            unused.push(reg.clone());
        }
    }
    for reg in &unused {
        context.types.remove(reg);
    }
}
