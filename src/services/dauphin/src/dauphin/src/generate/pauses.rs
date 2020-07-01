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
            InstructionType::Pause => {},
            _ => {
                let command = compiler_link.compile_instruction(instr,true)?.2;
                let name = format!("{:?}",instr).replace("\n","");
                if *time < 1. {
                    print!("execution time for {:?} is {:.3}ms\n",name,time);
                }
                timer += command.execution_time();
                while timer > 1. {
                    context.add(Instruction::new(InstructionType::Pause,vec![]));
                    timer -= 1.;
                }
                context.add(instr.clone())
            }
        }
    }
    context.phase_finished();
    Ok(())
}
