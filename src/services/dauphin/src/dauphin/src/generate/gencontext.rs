use std::fmt;
use std::mem::swap;
use super::instruction::{ Instruction, InstructionType };
use crate::model::{ Register, RegisterAllocator };
use crate::typeinf::{ MemberType, TypeModel };

pub struct GenContext {
    input_instrs: Vec<Instruction>,
    output_instrs: Vec<Instruction>,
    regalloc: RegisterAllocator,
    types: TypeModel
}

impl fmt::Debug for GenContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instr_str : Vec<String> = self.input_instrs.iter().map(|v| format!("{:?}",v)).collect();
        write!(f,"{:?}\n{}\n",self.types,instr_str.join(""))?;
        Ok(())
    }
}

impl GenContext {
    pub fn new() -> GenContext {
        GenContext {
            input_instrs: Vec::new(),
            output_instrs: Vec::new(),
            regalloc: RegisterAllocator::new(),
            types: TypeModel::new()
        }
    }

    pub fn get_instructions(&self) -> Vec<Instruction> {
        self.input_instrs.to_vec()
    }

    pub fn add_instruction(&mut self, instr: Instruction) {
        self.output_instrs.push(instr);
    }

    pub fn allocate_register(&mut self, type_: Option<&MemberType>) -> Register {
        let out = self.regalloc.allocate();
        if let Some(type_) = type_ {
            self.types.add(&out,type_);
        }
        out
    }

    pub fn add(&mut self, type_: &MemberType, itype: InstructionType, mut regs_in: Vec<Register>) -> Register {
        let dst = self.allocate_register(Some(type_));
        let mut regs = vec![dst];
        regs.append(&mut regs_in);
        self.output_instrs.push(Instruction::new(itype,regs));
        dst
    }

    pub fn phase_finished(&mut self) {
        swap(&mut self.input_instrs, &mut self.output_instrs);
        self.output_instrs = Vec::new();
    }

    pub fn xxx_types(&mut self) -> &mut TypeModel { &mut self.types }
}
