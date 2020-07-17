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

use std::collections::HashSet;
use super::gencontext::GenContext;
use crate::command::{ Instruction, InstructionType };

pub fn peephole_nil_append(context: &mut GenContext) -> Result<(),String> {
    let mut nil_regs = HashSet::new();
    let instrs = context.get_instructions();
    for instr in instrs.iter() {
        let mut instr = instr.clone();
        match instr.itype {
            InstructionType::Nil => {
                nil_regs.insert(instr.regs[0].clone());
            },
            InstructionType::Append => {
                if nil_regs.contains(&instr.regs[0]) {
                    instr = Instruction::new(InstructionType::Copy,vec![instr.regs[0].clone(),instr.regs[1].clone()]);
                }
                nil_regs.remove(&instr.regs[0]);
            },
            _ => {
                for idx in instr.itype.out_registers() {
                    nil_regs.remove(&instr.regs[idx]);
                }
            }
        }
        context.add(instr);
    }
    context.phase_finished();
    Ok(())
}

pub fn peephole_linenum_remove(context: &mut GenContext) -> Result<(),String> {
    let mut rev_instrs = context.get_instructions();
    rev_instrs.reverse();
    let mut seen_line = false;
    let mut out_instrs = vec![];
    for instr in rev_instrs.drain(..) {
        match instr.itype {
            InstructionType::LineNumber(_) => {
                if !seen_line {
                    out_instrs.push(instr);
                }
                seen_line = true;
            },
            _ => {
                out_instrs.push(instr);
                seen_line = false;
            }
        }
    }
    out_instrs.reverse();
    for instr in out_instrs.drain(..) {
        context.add(instr);
    }
    context.phase_finished();
    Ok(())
}
