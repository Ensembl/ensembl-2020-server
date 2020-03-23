use std::fmt;
use std::mem::swap;
use super::instruction::{ Instruction, InstructionType };
use crate::model::{ DefStore, Register, RegisterAllocator };
use crate::typeinf::{ ExpressionType, MemberType, TypeModel, Typing };

pub struct GenContext<'a> {
    defstore: &'a DefStore,
    input_instrs: Vec<Instruction>,
    output_instrs: Vec<Instruction>,
    regalloc: RegisterAllocator,
    types: TypeModel,
    typing: Typing
}

impl<'a> fmt::Debug for GenContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instr_str : Vec<String> = self.input_instrs.iter().map(|v| format!("{:?}",v)).collect();
        write!(f,"{:?}\n{}\n",self.types,instr_str.join(""))?;
        Ok(())
    }
}

impl<'a> GenContext<'a> {
    pub fn new(defstore: &'a DefStore) -> GenContext<'a> {
        GenContext {
            defstore,
            input_instrs: Vec::new(),
            output_instrs: Vec::new(),
            regalloc: RegisterAllocator::new(),
            types: TypeModel::new(),
            typing: Typing::new()
        }
    }

    pub fn get_instructions(&self) -> Vec<Instruction> {
        self.input_instrs.to_vec()
    }

    pub fn add_untyped(&mut self, instr: Instruction) -> Result<(),String> {
        self.typing.add(&instr.get_constraint(&self.defstore)?)?;
        self.output_instrs.push(instr);
        Ok(())
    }

    pub fn add_untyped_f(&mut self, itype: InstructionType, mut regs_in: Vec<Register>) -> Result<Register,String> {
        let dst = self.regalloc.allocate();
        let mut regs = vec![dst];
        regs.append(&mut regs_in);
        let instr = Instruction::new(itype,regs);
        self.typing.add(&instr.get_constraint(&self.defstore)?)?;
        self.output_instrs.push(instr);
        Ok(dst)
    }

    pub fn get_partial_type(&self, reg: &Register) -> ExpressionType {
        self.typing.get(reg)
    }

    pub fn generate_types(&mut self) {
        self.typing.to_model(&mut self.types);
    }

    pub fn add(&mut self, instr: Instruction) {
        self.output_instrs.push(instr);
    }

    pub fn allocate_register(&mut self, type_: Option<&MemberType>) -> Register {
        let out = self.regalloc.allocate();
        if let Some(type_) = type_ {
            self.types.add(&out,type_);
        }
        out
    }

    pub fn add_f(&mut self, type_: &MemberType, itype: InstructionType, mut regs_in: Vec<Register>) -> Register {
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
