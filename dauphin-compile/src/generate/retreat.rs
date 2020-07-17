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
use crate::command::{ InstructionType, Instruction };
use crate::lexer::LexerPosition;
use dauphin_interp::runtime::Register;

struct RetreatOutput {
    pos: Option<LexerPosition>,
    instrs: Vec<(Option<LexerPosition>,Instruction)>
}

impl RetreatOutput {
    fn new() -> RetreatOutput {
        RetreatOutput {
            pos: None,
            instrs: vec![]
        }
    }

    fn can_retreat(&self, jumper: &Instruction, jumped: &Instruction) -> bool {
        if let InstructionType::Pause(_) = &jumper.itype {
            return false;
        }
        match &jumped.itype {
            InstructionType::Pause(_) => { return false; },
            InstructionType::Call(_,true,_,_) => {
                if let InstructionType::Call(_,true,_,_) = jumper.itype {
                    return false;
                }
            },
            _ => {}
        }
        let jumper_regs : HashSet<Register> = jumper.regs.iter().cloned().collect();
        let jumped_regs : HashSet<Register> = jumped.regs.iter().cloned().collect();
        let jumped_out_idx : HashSet<usize> = jumped.itype.out_registers().iter().cloned().collect();
        let jumper_out_idx : HashSet<usize> = jumper.itype.out_registers().iter().cloned().collect();
        for (i,jumped_reg) in jumped.regs.iter().enumerate() {
            if jumped_out_idx.contains(&i) {
                if jumper_regs.contains(jumped_reg) {
                    return false;
                }
            }
        }
        for (i,jumper_reg) in jumper.regs.iter().enumerate() {
            if jumper_out_idx.contains(&i) {
                if jumped_regs.contains(jumper_reg) {
                    return false;
                }
            }
        }
        true
    }

    fn blocked_at(&self, instr: &Instruction) -> i32 {
        let mut pos = (self.instrs.len() as i32)-1;
        while pos >= 0 {
            if !self.can_retreat(instr,&self.instrs[pos as usize].1) {
                return pos;
            }
            pos -= 1;
        }
        -1
    }

    fn add(&mut self, instr: &Instruction) {
        let mut insert_at = None;
        match &instr.itype {
            InstructionType::LineNumber(pos) => {
                self.pos = Some(pos.clone());
                return;
            },
            InstructionType::Copy => {},
            _ => {
                let block = self.blocked_at(instr);
                if block < (self.instrs.len() as i32)-1 {
                    insert_at = Some((block+1) as usize);
                }
            }
        }
        let out = (self.pos.clone(),instr.clone());
        if let Some(pos) = insert_at {
            self.instrs.insert(pos,out);
        } else {
            self.instrs.push(out);
        }
    }

    fn finish(&self, context: &mut GenContext) {
        let mut line : Option<LexerPosition> = None;
        for (pos,instr) in &self.instrs {
            if let Some(cur_pos) = pos {
                let mut emit_line = true;
                if let Some(ref old_pos) = line {
                    if old_pos.filename() == cur_pos.filename() && old_pos.line() == cur_pos.line() {
                        emit_line = false;
                    }
                }
                if emit_line {
                    context.add(Instruction::new(InstructionType::LineNumber(cur_pos.clone()),vec![]));
                    line = Some(cur_pos.clone());
                }
            }
            context.add(instr.clone());
        }
    }
}

pub fn retreat(context: &mut GenContext) -> Result<(),String> {
    let mut retreat = RetreatOutput::new();
    let instrs = context.get_instructions();
    for instr in instrs.iter() {
        retreat.add(instr);
    }
    retreat.finish(context);
    context.phase_finished();
    Ok(())
}