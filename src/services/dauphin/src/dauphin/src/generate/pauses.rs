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
use crate::interp::{ InterpContext, InterpValue, CompilerLink, PreImageOutcome, numbers_to_indexes };
use crate::generate::{ Instruction, InstructionType };

pub fn pauses(compiler_link: &CompilerLink, resolver: &Resolver, context: &mut GenContext) -> Result<(),String> {
    let mut timer = 0.;
    for instr in &context.get_instructions() {
        match instr.itype {
            InstructionType::Pause => {},
            _ => {
                let command = compiler_link.compile_instruction(instr,true)?.2;
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
