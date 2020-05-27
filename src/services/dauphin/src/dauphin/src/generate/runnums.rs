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

use std::cell::RefCell;
use std::collections::{ HashMap, HashSet };
use std::rc::Rc;
use super::gencontext::GenContext;
use crate::model::Register;
use crate::interp::{ to_index, InterpContext, InterpValue, SuperCow, CompilerLink, CommandCompileSuite, Command, PreImageOutcome, numbers_to_indexes };
use crate::generate::{ Instruction, InstructionType };

pub struct PreImageContext<'a,'b> {
    suppressed: HashSet<Register>,
    compiler_link: CompilerLink,
    valid_registers: HashSet<Register>,
    context: InterpContext,
    gen_context: &'a mut GenContext<'b>,
    journal: Vec<Register>
}

impl<'a,'b> PreImageContext<'a,'b> {
    pub fn new(compiler_link: &CompilerLink, gen_context: &'a mut GenContext<'b>) -> Result<PreImageContext<'a,'b>,String> {
        Ok(PreImageContext {
            suppressed: HashSet::new(),
            compiler_link: compiler_link.clone(),
            valid_registers: HashSet::new(),
            context: InterpContext::new(),
            gen_context,
            journal: vec![]
        })
    }

    pub fn context(&mut self) -> &mut InterpContext { &mut self.context }

    pub fn add_instruction(&mut self, instr: &Instruction) -> Result<(),String> {
        let out_only = instr.itype.out_only_registers();
        for (i,reg) in instr.regs.iter().enumerate() {
            if !out_only.contains(&i) && self.suppressed.contains(reg) {
                self.make_constant(reg)?;
            }
            self.suppressed.remove(reg);
        }
        self.gen_context.add(instr.clone());
        Ok(())
    }

    pub fn set_reg_valid(&mut self, reg: &Register, valid: bool) {
        print!("valid {} {}\n",reg,valid);
        if valid {
            self.valid_registers.insert(*reg);
            self.journal.push(*reg);
        } else {
            self.valid_registers.remove(reg);
        }
    }

    pub fn get_reg_valid(&mut self, reg: &Register) -> bool {
        self.valid_registers.contains(reg)
    }

    fn unable_instr(&mut self, instr: &Instruction) {
        self.add_instruction(instr);
        let changing = instr.itype.out_registers();
        for idx in &changing {
            self.set_reg_valid(&instr.regs[*idx],false);
        }
    }

    fn long_constant<F,T>(&mut self, reg: &Register, values: &Vec<T>, mut cb: F) -> Result<(),String> where F: FnMut(Register,&T) -> Instruction {
        if values.len() == 1 {
            self.gen_context.add(cb(*reg,&values[0]));
        } else {
            self.gen_context.add(Instruction::new(InstructionType::Nil,vec![*reg]));
            for v in values {
                let inter = self.gen_context.allocate_register(None);
                self.gen_context.add(cb(inter,v));
                self.gen_context.add(Instruction::new(InstructionType::Append,vec![*reg,inter]));
            }
        }
        Ok(())
    }

    fn make_constant(&mut self, reg: &Register) -> Result<(),String> {
        // XXX don't copy the big ones
        let value = self.context().registers().get(reg).borrow().get_shared()?;
        match value.as_ref() {
            InterpValue::Empty => {
                self.gen_context.add(Instruction::new(InstructionType::Nil,vec![*reg]));
            },
            InterpValue::Indexes(indexes) => {
                self.gen_context.add(Instruction::new(InstructionType::Const(indexes.to_vec()),vec![*reg]));
            },
            InterpValue::Numbers(numbers) => {
                if let Some(indexes) = numbers_to_indexes(numbers).ok() {
                    self.gen_context.add(Instruction::new(InstructionType::Const(indexes.to_vec()),vec![*reg]));
                } else {
                    self.long_constant(reg,numbers,|r,n| {
                        Instruction::new(InstructionType::NumberConst(*n),vec![r])
                    })?;
                }
            },
            InterpValue::Boolean(bools) => {
                self.long_constant(reg,bools,|r,n| {
                    Instruction::new(InstructionType::BooleanConst(*n),vec![r])
                })?;
            },
            InterpValue::Strings(strings) => {
                self.long_constant(reg,strings,|r,n| {
                    Instruction::new(InstructionType::StringConst(n.clone()),vec![r])
                })?;
            },
            InterpValue::Bytes(bytes) => {
                self.long_constant(reg,bytes,|r,n| {
                    Instruction::new(InstructionType::BytesConst(n.clone()),vec![r])
                })?;
            },
        }
        Ok(())
    }

    fn preimage_instr(&mut self, instr: &Instruction) -> Result<(),String> {
        print!("instr {:?}\n",instr);
        let command = self.compiler_link.compile_instruction(instr)?.2;
        match command.preimage(self) ? {
            PreImageOutcome::Skip => {
                self.unable_instr(&instr);
            },
            PreImageOutcome::Replace(instrs) => {
                for instr in instrs {
                    self.preimage_instr(&instr)?;
                }                    
            },
            PreImageOutcome::Constant(regs) => {
                self.context().registers().commit();
                for reg in &regs {
                    self.suppressed.insert(*reg);
                }
            }
        }
        let (context,journal) = (&mut self.context,&mut self.journal);
        for reg in journal.drain(..) {
            print!("value {:?} {:?}\n",reg,context.registers().get(&reg).borrow().get_shared().expect(""));
        }
        self.context().registers().commit();
        Ok(())
    }

    pub fn preimage(&mut self) -> Result<(),String> {
        for instr in &self.gen_context.get_instructions() {
            self.preimage_instr(instr)?;
        }
        self.gen_context.phase_finished();
        Ok(())
    }
}

fn update_values(values: &mut HashMap<Register,Vec<usize>>, changing: &[usize], instr: &Instruction) {
    match &instr.itype {
        InstructionType::Nil => {
            values.insert(instr.regs[0],vec![]);
        },

        InstructionType::Copy => {
            if let Some(src) = values.get(&instr.regs[1]).cloned() {
                values.insert(instr.regs[0],src.to_vec());
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Append => {
            if let Some(src) = values.get(&instr.regs[1]) {
                let value = src.to_vec();
                values.get_mut(&instr.regs[0]).unwrap().extend(value.iter());
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::NumberConst(n) => {
            if let Some(v) = to_index(*n) {
                values.insert(instr.regs[0],vec![v]);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Const(nn) => {
            values.insert(instr.regs[0],nn.to_vec());
        },

        InstructionType::At => {
            if let Some(src) = values.get(&instr.regs[1]) {
                let mut value = vec![];
                for i in 0..src.len() {
                    value.push(i);
                }
                values.insert(instr.regs[0],value);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Filter => {
            if let (Some(src),Some(filter)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut f = filter.iter();
                for u in src {
                    if *f.next().unwrap() > 0 {
                        dst.push(*u);
                    }
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Run => {
            if let (Some(src),Some(filter)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut b_iter = filter.iter();
                for a in src.iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b as usize {
                        dst.push(a+i);
                    }
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::SeqFilter => {
            if let (Some(src),Some(start),Some(len)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2]),values.get(&instr.regs[3])) {
                let mut dst = vec![];
                let mut b_iter = len.iter();
                for a in start.iter() {
                    let b = b_iter.next().unwrap();
                    for i in 0..*b as usize {
                        dst.push(src[*a as usize+i]);
                    }
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::NumEq => {
            if let (Some(aa),Some(bb)) = (values.get(&instr.regs[1]),values.get(&instr.regs[2])) {
                let mut dst = vec![];
                let mut b_iter = bb.iter().cycle();
                for a in aa {
                    let b = b_iter.next().unwrap();
                    dst.push(if *a == *b {1} else {0});
                }
                values.insert(instr.regs[0],dst);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Length => {
            if let Some(src) = values.get(&instr.regs[1]).cloned() {
                values.insert(instr.regs[0],vec![src.len()]);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::Add => {
            if let (Some(dst),Some(delta)) = (values.get(&instr.regs[0]),values.get(&instr.regs[1])) {
                let mut out = vec![];
                for (i,input) in dst.iter().enumerate() {
                    out.push(input+delta[i%delta.len()]);
                }
                values.insert(instr.regs[0],out);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        InstructionType::SeqAt => {
            if let Some(src) = values.get(&instr.regs[1]).cloned() {
                let mut out = vec![];
                for b_val in &src {
                    for i in 0..*b_val as usize {
                        out.push(i);
                    }
                }
                values.insert(instr.regs[0],out);
            } else {
                values.remove(&instr.regs[0]);
            }
        },

        _ => {
            for idx in changing {
                values.remove(&instr.regs[*idx]);
            }
        }
    }
}

fn all_known(values: &HashMap<Register,Vec<usize>>, changing: &[usize], instr: &Instruction) -> bool {
    let mut out = true;
    for i in changing {
        if !values.contains_key(&instr.regs[*i]) {
            out = false;
        }
    }
    out
}

pub fn run_nums(compiler_link: &CompilerLink, context: &mut GenContext) -> Result<(),String> {
    let mut pic = PreImageContext::new(compiler_link,context)?;
    pic.preimage()?;
    return Ok(());
    let mut values : HashMap<Register,Vec<usize>> = HashMap::new();
    let mut suppressed = HashSet::new();
    for instr in &context.get_instructions() {
        let changing = instr.itype.out_registers();
        /* capture suppressed in/outs now as update_values will trample on them */
        let mut old_values : HashMap<Register,Vec<usize>> = HashMap::new();
        for reg in &instr.regs {
            if suppressed.contains(reg) {
                if let Some(old_value) = values.get(reg) {
                    old_values.insert(*reg,old_value.to_vec());
                }
            }
        }
        update_values(&mut values,&changing,instr);
        if all_known(&values,&changing,instr) && !instr.itype.self_justifying_call() {
            for i in changing {
                suppressed.insert(&instr.regs[i]);
            }
        } else {
            for reg in &instr.regs {
                if suppressed.contains(reg) {
                    if let Some(old_value) = old_values.remove(reg) {
                        context.add(Instruction::new(InstructionType::Const(old_value),vec![*reg]));
                    }
                    suppressed.remove(reg);
                }
            }
            for i in changing {
                suppressed.remove(&instr.regs[i]);
            }
            context.add(instr.clone());
        }

    }
    context.phase_finished();
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::Lexer;
    use crate::resolver::test_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::generate::prune::prune;
    use crate::interp::{ mini_interp, xxx_compiler_link };
    use super::super::linearize;
    use super::super::remove_aliases;

    #[test]
    fn runnums_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts,true).expect("codegen");
        let linker = xxx_compiler_link().expect("y");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}",context);
        run_nums(&linker,&mut context).expect("x");
        prune(&mut context);
        print!("RUN NUMS\n");
        print!("{:?}",context);
        let lines = format!("{:?}",context).as_bytes().iter().filter(|&&c| c == b'\n').count();
        print!("{}\n",lines);
        //assert!(lines<350);
        let (values,strings) = mini_interp(&mut context,&linker).expect("x");
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

    #[test]
    fn runnums2_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/runnums.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts,true).expect("codegen");
        let linker = xxx_compiler_link().expect("y");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}",context);
        run_nums(&linker,&mut context).expect("x");
        prune(&mut context);
        print!("RUN NUMS\n");
        print!("{:?}",context);
        let lines = format!("{:?}",context).as_bytes().iter().filter(|&&c| c == b'\n').count();
        print!("{}\n",lines);
    }

}