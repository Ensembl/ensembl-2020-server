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
use super::gencontext::GenContext;
use crate::resolver::Resolver;
use crate::model::Register;
use crate::interp::{ InterpContext, InterpValue, SuperCow, CompilerLink, CommandCompileSuite, Command, PreImageOutcome, numbers_to_indexes };
use crate::generate::{ Instruction, InstructionType };

#[derive(Clone,Hash,PartialEq,Eq)]
enum StealableValue {
    Empty,
    Indexes(Vec<usize>),
    Boolean(Vec<bool>),
}

impl StealableValue {
    fn new(iv: &InterpValue) -> Option<StealableValue> {
        match iv {
            InterpValue::Empty => Some(StealableValue::Empty),
            InterpValue::Indexes(x) => Some(StealableValue::Indexes(x.clone())),
            InterpValue::Boolean(b) => Some(StealableValue::Boolean(b.clone())),
            _ => None
        }
    }
}

pub struct PreImageContext<'a,'b> {
    resolver: &'a Resolver,
    steals: HashMap<StealableValue,Register>,
    rev_steals: HashMap<Register,StealableValue>,
    suppressed: HashSet<Register>,
    compiler_link: CompilerLink,
    valid_registers: HashSet<Register>,
    context: InterpContext,
    gen_context: &'a mut GenContext<'b>
}

impl<'a,'b> PreImageContext<'a,'b> {
    pub fn new(compiler_link: &CompilerLink, resolver: &'a Resolver, gen_context: &'a mut GenContext<'b>) -> Result<PreImageContext<'a,'b>,String> {
        Ok(PreImageContext {
            resolver,
            steals: HashMap::new(),
            rev_steals: HashMap::new(),
            suppressed: HashSet::new(),
            compiler_link: compiler_link.clone(),
            valid_registers: HashSet::new(),
            context: InterpContext::new(),
            gen_context
        })
    }

    pub fn context(&mut self) -> &mut InterpContext { &mut self.context }
    pub fn resolver(&self) -> &Resolver { &self.resolver }

    pub fn add_instruction(&mut self, instr: &Instruction) -> Result<(),String> {
        let out = instr.itype.out_registers();
        let out_only = instr.itype.out_only_registers();
        for (i,reg) in instr.regs.iter().enumerate() {
            let mut make = true;
            if !out_only.contains(&i) && self.suppressed.contains(reg) {
                let value = self.context().registers().get(reg).borrow().get_shared()?;
                if let Some(sv) = StealableValue::new(&value) {
                    if let Some(other_reg) = self.steals.get(&sv) {
                        self.gen_context.add(Instruction::new(InstructionType::Copy,vec![*reg,*other_reg]));
                        make = false;
                    }
                    self.steals.insert(sv.clone(),*reg);
                    self.rev_steals.insert(*reg,sv);
                }
                if make {
                    self.make_constant(reg)?;
                }
            }
            self.suppressed.remove(reg);
        }
        for (i,reg) in instr.regs.iter().enumerate() {
            if out.contains(&i) {
                if let Some(sv) = self.rev_steals.remove(reg) {
                    self.steals.remove(&sv);
                }
            }
        }
        self.gen_context.add(instr.clone());
        Ok(())
    }

    pub fn set_reg_valid(&mut self, reg: &Register, valid: bool) {
        if valid {
            self.valid_registers.insert(*reg);
        } else {
            self.valid_registers.remove(reg);
        }
    }

    pub fn get_reg_valid(&mut self, reg: &Register) -> bool {
        self.valid_registers.contains(reg)
    }

    fn unable_instr(&mut self, instr: &Instruction) -> Result<(),String> {
        self.add_instruction(instr)?;
        let changing = instr.itype.out_registers();
        for idx in &changing {
            self.set_reg_valid(&instr.regs[*idx],false);
        }
        Ok(())
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
        let command = self.compiler_link.compile_instruction(instr)?.2;
        match command.preimage(self) ? {
            PreImageOutcome::Skip => {
                self.unable_instr(&instr)?;
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

pub fn compile_run(compiler_link: &CompilerLink, resolver: &Resolver, context: &mut GenContext) -> Result<(),String> {
    let mut pic = PreImageContext::new(compiler_link,resolver,context)?;
    pic.preimage()?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call::call;
    use super::super::simplify::simplify;
    use crate::lexer::Lexer;
    use crate::resolver::test_resolver;
    use crate::parser::{ Parser };
    use crate::generate::prune::prune;
    use crate::interp::{ mini_interp, xxx_compiler_link, xxx_test_config };
    use super::super::codegen::generate_code;
    use super::super::linearize::linearize;
    use super::super::dealias::remove_aliases;

    #[test]
    fn runnums_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(&resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        let linker = xxx_compiler_link().expect("y");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}",context);
        compile_run(&linker,&resolver,&mut context).expect("x");
        prune(&mut context);
        print!("RUN NUMS\n");
        print!("{:?}",context);
        let lines = format!("{:?}",context).as_bytes().iter().filter(|&&c| c == b'\n').count();
        print!("{}\n",lines);
        assert!(lines<350);
        let config = xxx_test_config();
        let (values,strings) = mini_interp(&mut context.get_instructions(),&linker,&config).expect("x");
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

    #[test]
    fn runnums2_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(&resolver);
        lexer.import("test:codegen/runnums.dp").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        let linker = xxx_compiler_link().expect("y");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}",context);
        compile_run(&linker,&resolver,&mut context).expect("x");
        prune(&mut context);
        print!("RUN NUMS\n");
        print!("{:?}",context);
        let lines = format!("{:?}",context).as_bytes().iter().filter(|&&c| c == b'\n').count();
        print!("{}\n",lines);
    }

}