use std::collections::{ HashMap, HashSet };
use crate::generate::instruction::{ Instruction, InstructionType };
use crate::model::Register;
use super::gencontext::GenContext;

struct Value(usize);

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

    fn discard_old(&mut self, register: &Register) -> Option<(Register,Register)> {
        print!("  discarding {:?}\n",register);
        if let Some(value) = self.reg_value.remove(register) {
            print!("    value {:?}\n",value);
            if let Some(spares) = self.spare_regs.get_mut(&value) {
                print!("      spare (main={:?})\n",self.value_reg.get(&value).unwrap());
                if spares.remove(register) {
                    return Some((*register,*self.value_reg.get(&value).unwrap()));
                }
            }
            if let Some(main_register) = self.value_reg.get(&value) {
                if main_register == register {
                    print!("      main\n");
                    self.reg_value.remove(register);
                    let next_reg = self.spare_regs.get(&value).as_ref().unwrap().iter().next().cloned();
                    if let Some(next_reg) = next_reg {
                        print!("      next = {:?}\n",next_reg);
                        self.value_reg.insert(value,next_reg);
                        self.spare_regs.get_mut(&value).as_mut().unwrap().remove(&next_reg);
                        print!("      copy {:?} {:?}\n",next_reg,*register);
                        return Some((next_reg,*register));
                    }
                }
            }
        }
        None
    }

    fn new_value(&mut self, register: &Register) {
        let new_value = self.next_value;
        self.next_value += 1;
        print!("  new value for {:?} = {:?}\n",register,new_value);
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

    fn invalidate(&mut self, register: &Register) -> Option<(Register,Register)> {
        let out = if let Some(old_value) = self.reg_value.get(register) {
            self.discard_old(register)
        } else {
            None
        };
        let new_value = self.next_value;
        self.next_value += 1;
        print!("  new value for {:?} = {:?}\n",register,new_value);
        self.reg_value.insert(*register,new_value);
        self.value_reg.insert(new_value,*register);
        self.spare_regs.insert(new_value,HashSet::new());
        out
    }

    fn alias(&mut self, alias: &Register, main: &Register) {
        print!("  alias {:?} -> {:?}\n",alias,main);
        if let Some(ref mut value) = self.reg_value.get_mut(main).cloned() {
            print!("    value={:?}\n",value);
            self.spare_regs.get_mut(value).as_mut().unwrap().insert(*alias);
            self.reg_value.insert(*alias,*value);
        }
    }

    fn get_main(&mut self, target: &Register) -> Register {
        if self.reg_value.get(target).is_none() {
            self.invalidate(target);
        }
        *self.value_reg.get(self.reg_value.get(target).unwrap()).unwrap()
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
        print!("instruction: {:?}\n",instr);
        match instr.itype {
            InstructionType::Copy => {
                values.alias(&instr.regs[0],&instr.regs[1]);
            },
            _ => {
                /* get list of registers which are mutated by call */
                let mutating_regs = if instr.itype.self_justifying_call() {
                    instr.regs.clone()
                } else {
                    instr.itype.justifying_registers(context.get_defstore())
                        .iter().map(|x| instr.regs[*x]).collect()
                };
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
                print!("emit {:?}\n",Instruction::new(instr.itype.clone(),new_regs.clone()));
                context.add(Instruction::new(instr.itype,new_regs));
            }
        }
    }
    context.phase_finished();
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::interp::mini_interp;
    use super::super::linearize;
    use super::super::remove_aliases;
    use super::super::prune;

    #[test]
    fn cow_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        prune(&mut context);
        print!("{:?}\n",context);
        copy_on_write(&mut context);
        print!("{:?}\n",context);
        prune(&mut context);
        let (_prints,values,strings) = mini_interp(&defstore,&mut context);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}