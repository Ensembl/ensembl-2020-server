/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::fmt;
use std::mem::swap;
use crate::command::{ Instruction, InstructionType };
use crate::model::{ DefStore, RegisterAllocator };
use crate::typeinf::{ ExpressionType, MemberType, TypeModel, Typing, get_constraint };
use dauphin_interp::runtime::Register;

pub struct GenContext<'a> {
    defstore: &'a DefStore,
    input_instrs: Vec<(Instruction,f64)>,
    output_instrs: Vec<(Instruction,f64)>,
    regalloc: RegisterAllocator,
    types: TypeModel,
    typing: Typing
}

impl<'a> fmt::Debug for GenContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instr_str : Vec<String> = self.input_instrs.iter().map(|v| format!("{:?}",v.0)).collect();
        write!(f,"{}\n",instr_str.join(""))?;
        Ok(())
    }
}

impl<'a> GenContext<'a> {
    pub fn new(defstore: &'a DefStore) -> GenContext<'a> {
        GenContext {
            defstore,
            input_instrs: Vec::new(),
            output_instrs: Vec::new(),
            regalloc: RegisterAllocator::new(0),
            types: TypeModel::new(),
            typing: Typing::new()
        }
    }

    pub fn get_defstore(&self) -> &DefStore { self.defstore }

    pub fn get_instructions(&self) -> Vec<Instruction> {
        self.input_instrs.iter().map(|x| x.0.clone()).collect()
    }

    pub fn get_timed_instructions(&self) -> Vec<(Instruction,f64)> {
        self.input_instrs.to_vec()
    }

    pub fn add_untyped(&mut self, instr: Instruction) -> Result<(),String> {
        self.typing.add(&get_constraint(&instr,&self.defstore)?).map_err(|x| format!("{} while adding {:?}",x,instr))?;
        self.output_instrs.push((instr,0.));
        Ok(())
    }

    pub fn add_untyped_f(&mut self, itype: InstructionType, mut regs_in: Vec<Register>) -> Result<Register,String> {
        let dst = self.regalloc.allocate();
        let mut regs = vec![dst];
        regs.append(&mut regs_in);
        let instr = Instruction::new(itype,regs);
        self.add_untyped(instr)?;
        Ok(dst)
    }

    pub fn get_partial_type(&self, reg: &Register) -> ExpressionType {
        self.typing.get(reg)
    }

    pub fn generate_types(&mut self) {
        self.typing.to_model(&mut self.types);
    }

    pub fn add(&mut self, instr: Instruction) {
        self.output_instrs.push((instr,0.));
    }

    pub fn add_timed(&mut self, instr: Instruction, time: f64) {
        self.output_instrs.push((instr,time));
    }

    pub fn allocate_register(&mut self, type_: Option<&MemberType>) -> Register {
        let out = self.regalloc.allocate();
        if let Some(type_) = type_ {
            self.types.add(&out,type_);
        }
        out
    }

    pub fn phase_finished(&mut self) {
        swap(&mut self.input_instrs, &mut self.output_instrs);
        self.output_instrs = Vec::new();
    }

    pub fn xxx_types(&mut self) -> &mut TypeModel { &mut self.types }
}
