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

use std::collections::{ HashMap, HashSet };
use crate::generate::instruction::{ Instruction, InstructionType };
use crate::model::Register;
use super::gencontext::GenContext;

#[derive(Debug)]
struct CurrentValues {
    next_value: usize,
    reg_value: HashMap<Register,usize>,
    value_reg: HashMap<usize,Register>,
    spare_regs: HashMap<usize,HashSet<Register>>
}

impl CurrentValues {
    fn new() -> CurrentValues {
        CurrentValues {
            next_value: 0,
            reg_value: HashMap::new(),
            value_reg: HashMap::new(),
            spare_regs: HashMap::new()
        }
    }

    fn new_value(&mut self, register: &Register) {
        let new_value = self.next_value;
        self.next_value += 1;
        self.reg_value.insert(*register,new_value);
        self.value_reg.insert(new_value,*register);
        self.spare_regs.insert(new_value,HashSet::new());       
    }

    fn promote_spare(&mut self, register: &Register) -> Option<(Register,Register)> {
        if let Some(value) = self.reg_value.get(register) {
            if let Some(spares) = self.spare_regs.get_mut(&value) {
                if spares.remove(register) {
                    let main_reg = *self.value_reg.get(value).unwrap();
                    self.new_value(register);
                    return Some((*register,main_reg))
                }
            }
        }
        None
    }

    fn shadowed(&mut self, register: &Register) -> bool {
        if let Some(value) = self.reg_value.get(register) {
            let main_reg = *self.value_reg.get(value).unwrap();
            if main_reg == *register {
                if self.spare_regs.get(value).is_some() {
                    return true;
                }
            }
        }
        false
    }

    fn invalidate_main(&mut self, register: &Register) -> Option<(Register,Register)> {
        if let Some(value) = self.reg_value.get(register) {
            let main_reg = *self.value_reg.get(value).unwrap();
            let mut candidate = None;
            if main_reg == *register {
                if let Some(spares) = self.spare_regs.get(value) {
                    candidate = spares.iter().next().cloned();
                }
            }
            if let Some(candidate) = candidate {
                self.spare_regs.get_mut(value).unwrap().remove(&candidate);
                self.value_reg.insert(*value,candidate);
                self.new_value(register);
                return Some((candidate,main_reg));
            }
        }
        None
    }

    fn alias(&mut self, alias: &Register, main: &Register) {
        if alias == main { return; }
        if let Some(ref mut value) = self.reg_value.get_mut(main).cloned() {
            self.spare_regs.get_mut(value).as_mut().unwrap().insert(*alias);
            self.reg_value.insert(*alias,*value);
        }
    }

    fn get_main(&mut self, target: &Register) -> Register {
        if self.reg_value.get(target).is_none() {
            self.new_value(target);
        }
        *self.value_reg.get(self.reg_value.get(target).unwrap()).unwrap()
    }
}

fn process_instruction(context: &mut GenContext, instr: &Instruction, values: &mut CurrentValues, consts: Option<&mut ConstMatcher>) {
    /* get list of registers which are mutated by call */
    let mutating_regs = instr.itype.out_registers()
            .iter().map(|x| instr.regs[*x]).collect::<Vec<_>>();
    /* If any mutating regs are spare for some value, they need their own value now */
    for reg in &mutating_regs {
        if let Some((dst,src)) = values.promote_spare(&reg) {
            context.add(Instruction::new(InstructionType::Copy,vec![dst,src]));
        }
    }
    /* Build list of registers to use when we eventually call */
    let mut new_regs = Vec::new();
    for old_reg in instr.regs.iter() {
        let new_reg = values.get_main(old_reg);
        new_regs.push(new_reg);

    }
    /* If any mutating regs are main for some value, they are going to change, so any spares need new value */
    for reg in &mutating_regs {
        if let Some((dst,src)) = values.invalidate_main(&reg) {
            context.add(Instruction::new(InstructionType::Copy,vec![dst,src]));
        }
    }
    /* Do it! */
    context.add(Instruction::new(instr.itype.clone(),new_regs));
    /* Any constants we thought we had, we don't any more! */
    if let Some(consts) = consts {
        for reg in &mutating_regs {
            consts.remove(reg);
        }
    }
}

/* Note: copy_on_write never removes a value for being dead. It can copy values that are never used again to avoid them dying.
 * To complete the job it needs: 1. another prune (to eliminate copies-to-nothing) and 2. a call to reuse_dead (to relabel
 * copies with a source that's never used again).
 */
pub fn copy_on_write(context: &mut GenContext) {
    let mut values = CurrentValues::new();
    let instrs = context.get_instructions();
    for instr in instrs {
        match &instr.itype {
            InstructionType::Copy => {
                if let Some((dst,src)) = values.invalidate_main(&instr.regs[0]) {
                    context.add(Instruction::new(InstructionType::Copy,vec![dst,src]));
                }
                values.alias(&instr.regs[0],&instr.regs[1]);
            },
            _ => {
                process_instruction(context,&instr,&mut values,None);
            }
        }
    }
    context.phase_finished();
}

#[derive(Debug)]
struct ConstMatcher {
    reg_val: HashMap<Register,Vec<usize>>,
    val_regs: HashMap<Vec<usize>,HashSet<Register>>
}

impl ConstMatcher {
    fn new() -> ConstMatcher {
        ConstMatcher {
            reg_val: HashMap::new(),
            val_regs: HashMap::new()
        }
    }

    fn remove(&mut self, register: &Register) {
        if let Some(h) = self.reg_val.get(register).cloned() {
            self.reg_val.remove(register);
            self.val_regs.get_mut(&h).unwrap().remove(register);
        }
    }

    fn add(&mut self, register: &Register, value: &[usize]) {
        self.remove(register);
        self.val_regs.entry(value.to_vec()).or_insert_with(|| HashSet::new()).insert(*register);
        self.reg_val.insert(*register,value.to_vec());
    }

    fn get(&mut self, value: &[usize]) -> Option<Register> {
        if let Some(regs) = self.val_regs.get(value) {
            return regs.iter().next().cloned();
        }
        None
    }

    fn copy(&mut self, dst: &Register, src: &Register) {
        self.remove(dst);
        if let Some(h) = self.reg_val.get(&src).cloned() {
            self.val_regs.entry(h.clone()).or_insert_with(|| HashSet::new()).insert(*dst);
            self.reg_val.insert(*dst,h.to_vec());
        }
    }
}

pub fn reuse_const(context: &mut GenContext) {
    let mut values = CurrentValues::new();
    let mut consts = ConstMatcher::new();
    let instrs = context.get_instructions();
    for instr in instrs {
        match &instr.itype {
            InstructionType::Const(nn) => {
                let mut skip = false;
                if let Some(reg) = consts.get(nn) {
                    if !values.shadowed(&reg) {
                        values.alias(&instr.regs[0],&reg);
                        consts.copy(&instr.regs[0],&reg);
                        skip = true;
                    }
                }
                if !skip {
                    process_instruction(context,&instr,&mut values, Some(&mut consts));
                    consts.add(&instr.regs[0],nn);
                }
            },
            _ => {
                process_instruction(context,&instr,&mut values, Some(&mut consts));
            }
        }
    }
    context.phase_finished();
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::simplify::simplify;
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_librarysuite_builder };
    use super::super::dealias::remove_aliases;
    use super::super::prune::prune;
    use super::super::compilerun::compile_run;
    use super::super::generate::generate;
    use super::super::codegen::generate_code;
    use super::super::call::call;
    use super::super::linearize::linearize;

    #[test]
    fn cow_smoke() {
        let config = xxx_test_config();
        let resolver = common_resolver(&config).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        let linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        compile_run(&linker,&resolver,&mut context).expect("m");
        prune(&mut context);
        print!("{:?}\n",context);
        copy_on_write(&mut context);
        print!("{:?}\n",context);
        prune(&mut context);
        let (_,strings) = mini_interp(&context.get_instructions(),&linker,&config).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

    #[test]
    fn reuse_consts_smoke() {
        let config = xxx_test_config();
        let resolver = common_resolver(&config).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&linker,&config).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}