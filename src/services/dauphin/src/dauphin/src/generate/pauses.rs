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
use super::compilerun::compile_run;
use crate::resolver::Resolver;
use crate::model::Register;
use crate::interp::{ InterpContext, InterpValue, CompilerLink, PreImageOutcome, numbers_to_indexes };
use crate::generate::{ Instruction, InstructionType };

pub fn pauses(compiler_link: &CompilerLink, resolver: &Resolver, context: &mut GenContext) -> Result<(),String> {
    /* force compilerun to ensure timed instructions */ // XXX only if absent
    compile_run(compiler_link,resolver,context)?;
    let mut timer = 0.;
    for (instr,time) in &context.get_timed_instructions() {
        match instr.itype {
            InstructionType::Pause(true) => {
                context.add(instr.clone());
                timer = 0.;
            },
            InstructionType::Pause(false) => {},
            _ => {
                let command = compiler_link.compile_instruction(instr,true)?.2;
                let name = format!("{:?}",instr).replace("\n","");
                print!("execution time for {:?} is {:.3}ms (before timer={:.3}ms)\n",name,time,timer);
                timer += time;
                if timer >= 1. {
                    context.add(Instruction::new(InstructionType::Pause(false),vec![]));
                    print!("added pause\n");
                    timer = *time;
                }
                context.add(instr.clone())
            }
        }
    }
    context.phase_finished();
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

    fn pause_check(filename: &str) -> bool {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import(&format!("search:codegen/{}",filename)).expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        let mut seen_force_pause = false;
        for instr in &instrs {
            if seen_force_pause {
                print!("AFTER {:?}",instr);
                return if let InstructionType::Pause(_) = &instr.itype {
                    true
                } else {
                    false
                };
            }
            if let InstructionType::Pause(true) = &instr.itype {
                seen_force_pause = true;
            }
        }
        false
    }

    #[test]
    fn pause() {
        assert!(pause_check("pause"));
        assert!(!pause_check("no-pause"));
    }
}