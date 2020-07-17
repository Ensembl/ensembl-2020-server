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

use std::collections::HashMap;
use super::gencontext::GenContext;
use crate::command::{ InstructionType };
use dauphin_interp::runtime::Register;

struct RegEquiv {
    next_set: usize,
    reg_set: HashMap<Register,usize>,
    set_regs: HashMap<usize,Vec<Register>>
}

impl RegEquiv {
    fn new() -> RegEquiv {
        RegEquiv {
            next_set: 0,
            reg_set: HashMap::new(),
            set_regs: HashMap::new()
        }
    }

    fn unknown(&mut self, reg: &Register) {
        //print!("{:?} is now unknown\n",reg);
        if let Some(set) = self.reg_set.remove(reg) {
            let mut remove = false;
            if let Some(regs) = self.set_regs.get_mut(&set) {
                if let Some(pos) = regs.iter().position(|x| *x == *reg) {
                    regs.remove(pos);
                }
                if regs.len() == 0 {
                    remove = true;
                }
            }
            if remove {
                self.set_regs.remove(&set);
            }
        }
    }

    fn equiv(&mut self, moving: &Register, to_match: &Register) {
        self.unknown(moving);
        //print!("{:?} is now equivalent to {:?}\n",moving,to_match);
        let set = match self.reg_set.get(to_match) {
            Some(id) => *id,
            None => {
                let new_id = self.next_set;
                self.next_set += 1;
                self.set_regs.insert(new_id,vec![to_match.clone()]);
                self.reg_set.insert(to_match.clone(),new_id);
                new_id
            }
        };
        if let Some(regs) = self.set_regs.get_mut(&set) {
            //print!("A {:?} is now equivalent to {:?} setc={:?} {:?}\n",moving,to_match,set,regs);
            regs.push(*moving);
            self.reg_set.insert(moving.clone(),set);
        }
    }

    fn map(&self, reg: &Register) -> Register {
        if let Some(set) = self.reg_set.get(reg) {
            if let Some(regs) = self.set_regs.get(set) {
                if let Some(first) = regs.first() {
                    return first.clone();
                }
            }
        }
        reg.clone()
    }
}

pub fn use_earliest_regs(context: &mut GenContext) -> Result<(),String> {
    let mut equivs = RegEquiv::new();
    let instrs = context.get_instructions();
    /* Flag copies where source is last mention of a variable with appropriate rewrite */
    for instr in instrs.iter() {
        let mut instr = instr.clone();
        //print!("INSTR {:?}",instr);
        match instr.itype {
            InstructionType::Copy => {
                equivs.equiv(&instr.regs[0],&instr.regs[1]);
            },
            _ => {
                let out = instr.itype.out_registers();
                let mut new_regs = vec![];
                for (i,reg) in instr.regs.iter().enumerate() {
                    if out.contains(&i) {
                        new_regs.push(reg.clone());
                    } else {
                        //print!("{:?} maps to {:?}\n",reg,equivs.map(reg));
                        new_regs.push(equivs.map(reg).clone());
                    }
                }
                for (i,_reg) in instr.regs.iter().enumerate() {
                    if out.contains(&i) {
                        equivs.unknown(&instr.regs[i]);
                    }
                }
                instr.regs = new_regs;
            }
        }
        context.add(instr.clone());
    }
    context.phase_finished();
    Ok(())
}
