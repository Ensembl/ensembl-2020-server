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
use crate::model::{ Register, RegisterAllocator, DFloat };
use crate::interp::{ InterpContext, InterpValue, CompilerLink, PreImageOutcome, numbers_to_indexes };
use crate::generate::{ Instruction, InstructionType };

pub struct PreImageContext<'a,'b> {
    resolver: &'a Resolver,
    reg_sizes: HashMap<Register,usize>,
    compiler_link: CompilerLink,
    valid_registers: HashSet<Register>,
    context: InterpContext,
    gen_context: &'a mut GenContext<'b>,
    regalloc: RegisterAllocator
}

impl<'a,'b> PreImageContext<'a,'b> {
    pub fn new(compiler_link: &CompilerLink, resolver: &'a Resolver, gen_context: &'a mut GenContext<'b>) -> Result<PreImageContext<'a,'b>,String> {
        let mut max_reg = 0;
        for instr in gen_context.get_instructions() {
            for reg in &instr.regs {
                if reg.0 > max_reg { max_reg = reg.0; }
            }
        }
        Ok(PreImageContext {
            resolver,
            reg_sizes: HashMap::new(),
            compiler_link: compiler_link.clone(),
            valid_registers: HashSet::new(),
            context: compiler_link.new_context(),
            gen_context,
            regalloc: RegisterAllocator::new(max_reg+1)
        })
    }

    pub fn context(&self) -> &InterpContext { &self.context }
    pub fn context_mut(&mut self) -> &mut InterpContext { &mut self.context }
    pub fn resolver(&self) -> &Resolver { &self.resolver }

    pub fn new_register(&self) -> Register { self.regalloc.allocate() }

    fn commit(&mut self) -> Result<(),String> {
        let regs = self.context_mut().registers_mut().commit();
        for reg in &regs {
            if self.is_reg_valid(reg) {
                let len = self.context_mut().registers_mut().get(reg).borrow().get_shared()?.len();
                self.set_reg_size(reg,Some(len));
            }
        }
        Ok(())
    }

    pub fn add_instruction(&mut self, instr: &Instruction) -> Result<(),String> {
        self.add(instr.clone())?;
        self.commit()
    }

    fn set_reg_valid(&mut self, reg: &Register) -> Result<(),String> {
        self.valid_registers.insert(*reg);
        Ok(())
    }

    pub fn set_reg_invalid(&mut self, reg: &Register) {
        self.valid_registers.remove(reg);
    }

    pub fn set_reg_size(&mut self, reg: &Register, size: Option<usize>) {
        //print!("set_reg_size({:?},{:?})\n",reg,size);
        if let Some(size) = size {
            self.reg_sizes.insert(reg.clone(),size);
        } else {
            self.reg_sizes.remove(reg);
        }
    }

    pub fn get_reg_size(&self, reg: &Register) -> Option<usize> { self.reg_sizes.get(reg).map(|x| *x) }

    pub fn is_reg_valid(&self, reg: &Register) -> bool {
        self.valid_registers.contains(reg)
    }

    fn unable_instr(&mut self, instr: &Instruction, sizes: &[(Register,usize)]) -> Result<(),String> {
        //let name = format!("{:?}",instr).replace("\n","");
        //print!("unable {:?} {:?}\n",name,sizes);
        self.add_instruction(instr)?;
        let changing = instr.itype.out_registers();
        for idx in &changing {
            self.set_reg_invalid(&instr.regs[*idx]);
            self.set_reg_size(&instr.regs[*idx],None);
        }
        for (reg,size) in sizes {
            self.set_reg_size(reg,Some(*size));
        }
        Ok(())
    }

    fn long_constant<F,T>(&mut self, reg: &Register, values: &Vec<T>, mut cb: F) -> Result<(),String> where F: FnMut(Register,&T) -> Instruction {
        if values.len() == 1 {
            self.add(cb(*reg,&values[0]))?;
        } else {
            self.add(Instruction::new(InstructionType::Nil,vec![*reg]))?;
            for v in values {
                let inter = self.gen_context.allocate_register(None);
                self.add(cb(inter,v))?;
                self.add(Instruction::new(InstructionType::Append,vec![*reg,inter]))?;
            }
        }
        Ok(())
    }

    fn add(&mut self, instr: Instruction) -> Result<(),String> {
        let command = self.compiler_link.compile_instruction(&instr,true)?.2;
        let time = command.execution_time(&self);
        self.gen_context.add_timed(instr,time);
        Ok(())
    }

    fn make_constant(&mut self, reg: &Register) -> Result<(),String> {
        // XXX don't copy the big ones
        let value = self.context_mut().registers_mut().get(reg).borrow().get_shared()?;
        match value.as_ref() {
            InterpValue::Empty => {
                self.add(Instruction::new(InstructionType::Nil,vec![*reg]))?;
            },
            InterpValue::Indexes(indexes) => {
                self.add(Instruction::new(InstructionType::Const(indexes.to_vec()),vec![*reg]))?;
            },
            InterpValue::Numbers(numbers) => {
                if let Some(indexes) = numbers_to_indexes(numbers).ok() {
                    self.add(Instruction::new(InstructionType::Const(indexes.to_vec()),vec![*reg]))?;
                } else {
                    self.long_constant(reg,numbers,|r,n| {
                        Instruction::new(InstructionType::NumberConst(DFloat::new(*n)),vec![r])
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
        //print!("{:?}",instr);
        let command = self.compiler_link.compile_instruction(instr,true)?.2;
        match command.preimage(self)? {
            PreImageOutcome::Skip(sizes) => {
                self.unable_instr(&instr,&sizes)?;
            },
            PreImageOutcome::Replace(instrs) => {
                for instr in instrs {
                    self.preimage_instr(&instr)?;
                }                    
            },
            PreImageOutcome::Constant(regs) => {
                for reg in &regs {
                    self.set_reg_valid(reg)?;
                }
                self.commit()?;
                for reg in &regs {
                    self.make_constant(reg)?;
                }
            }
        }
        self.commit()?;
        Ok(())
    }

    pub fn preimage(&mut self) -> Result<(),String> {
        for instr in &self.gen_context.get_instructions() {
            let time = self.preimage_instr(instr).map_err(|msg| {
                let line = self.context().get_line_number();
                if line.1 != 0 {
                    format!("{} at {}:{}",msg,line.0,line.1)
                } else {
                    msg.to_string()
                }
            })?;
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
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::prune::prune;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_librarysuite_builder };
    use super::super::codegen::generate_code;
    use super::super::linearize::linearize;
    use super::super::dealias::remove_aliases;
    use crate::generate::generate;

    #[test]
    fn runnums_smoke() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
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
        let (values,strings) = mini_interp(&mut context.get_instructions(),&mut linker,&config,"main").expect("x");
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

    #[test]
    fn runnums2_smoke() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/runnums").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
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

    #[test]
    fn size_hint() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/size-hint").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let (_,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        assert_eq!(vec!["hello world!", "1", "1", "3", "2", "2", "1000000000", "1000000000", "1000000000", "1000000000", "1000000000", "10", "10", "10", "1", "11", "11", "11"],strings);
        print!("{:?}\n",strings);
    }
}