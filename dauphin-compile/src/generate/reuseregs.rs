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
use crate::command::{ InstructionType, Instruction };
use dauphin_interp::runtime::Register;

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
struct UnknownValue {
    line: usize,
    position: usize
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
struct KnownValue {
    itype: InstructionType,
    line: Option<usize>,
    position: usize,
    inputs: Vec<ValueId>
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
struct ValueId(usize);

struct ValueStore {
    next_id: usize,
    values: HashMap<SavedValue,ValueId>
}

impl ValueStore {
    fn new() -> ValueStore {
        ValueStore {
            next_id: 0,
            values: HashMap::new()
        }
    }

    fn lookup(&mut self, value: &SavedValue) -> ValueId {
        if let Some(id) = self.values.get(value) {
            return id.clone();
        }
        let id = ValueId(self.next_id);
        //print!("NEW VALUE {:?} <- {:?}\n",id,value);
        self.next_id += 1;
        self.values.insert(value.clone(),id.clone());
        id
    }
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
enum SavedValue {
    Known(KnownValue),
    UnknownValue(UnknownValue)
}

struct RegisterValues {
    reg_to_val: HashMap<Register,ValueId>,
    val_to_reg: HashMap<ValueId,Vec<Register>>
}

impl RegisterValues {
    fn new() -> RegisterValues {
        RegisterValues {
            reg_to_val: HashMap::new(),
            val_to_reg: HashMap::new()
        }
    }

    fn insert(&mut self, reg: &Register, kv: &ValueId) {
        //print!("set({:?},{:?})\n",reg,kv);
        /* remove old value */
        if let Some(old_kv) = self.reg_to_val.get(reg) {
            if let Some(regs) = self.val_to_reg.get_mut(old_kv) {
                if let Some(pos) = regs.iter().position(|x| *x == *reg) {
                    regs.remove(pos);
                }
            }
        }
        /* insert new value */
        self.reg_to_val.insert(reg.clone(),kv.clone());
        self.val_to_reg.entry(kv.clone()).or_insert(vec![]).push(reg.clone());
    }

    fn get_by_reg(&self, reg: &Register) -> Option<&ValueId> {
        self.reg_to_val.get(reg)
    }

    fn get_by_val(&self, id: &ValueId) -> Option<Register> {
        self.val_to_reg.get(id).and_then(|x| x.first()).cloned()
    }
}

/* Relabel instead of copying from sources which are never reused. Recurse this until no change */
pub fn reuse_regs(context: &mut GenContext) -> Result<(),String> {
    let mut saved = ValueStore::new();
    let mut map = RegisterValues::new();
    let instrs = context.get_instructions();
    for (line,instr) in instrs.iter().enumerate() {
        //print!("REUSE {:?}\n",instr);
        let out = instr.itype.out_registers();
        let out_only = instr.itype.out_only_registers();
        let impure = match instr.itype {
            InstructionType::Call(_,true,_,_) => true,
            _ => false
        };
        match instr.itype {
            InstructionType::Copy => {
                if let Some(val) = map.get_by_reg(&instr.regs[1]).cloned() {
                    map.insert(&instr.regs[0],&val);
                    context.add(instr.clone());
                } else {
                    Err("reference to missing value")?
                }
            },
            InstructionType::LineNumber(_) | InstructionType::Pause(_) => {
                context.add(instr.clone());
            },
            _ => {
                let mut inputs = Some(vec![]);
                let rep_line = if impure { Some(line) } else { None };
                for (i,reg) in instr.regs.iter().enumerate() {
                    if !out_only.contains(&i) {
                        if let Some(val) = map.get_by_reg(reg) {
                            inputs.as_mut().unwrap().push(val.clone());
                        } else {
                            inputs = None;
                            break;
                        }
                    }
                }
                let mut outputs = None;
                if !impure {
                    outputs = Some(vec![]);
                    for (i,_) in instr.regs.iter().enumerate() {
                        if out.contains(&i) {
                            let kv = if let Some(ref inputs) = inputs {
                                SavedValue::Known(KnownValue {
                                    itype: instr.itype.clone(),
                                    position: i,
                                    line: rep_line,
                                    inputs: inputs.clone()
                                })
                            } else {
                                SavedValue::UnknownValue(UnknownValue {
                                    line: line,
                                    position: i
                                })
                            };
                            let id = saved.lookup(&kv);
                            if let Some(reg) = map.get_by_val(&id) {
                                outputs.as_mut().unwrap().push((reg,id));
                            } else {
                                outputs = None;
                                break;
                            }
                        }
                    }
                }
                if let Some(mut mapping) = outputs {
                    /* hit, replace with copies */
                    let mut srcs = mapping.drain(..);
                    for (i,_) in instr.regs.iter().enumerate() {
                        if out.contains(&i) {
                            let (reg,id) = srcs.next().unwrap();
                            if instr.regs[i].clone() != reg {
                                context.add(Instruction::new(InstructionType::Copy,vec![instr.regs[i].clone(),reg]));
                                map.insert(&instr.regs[i],&id);
                            }
                        }
                    }
                } else {
                    /* no hit, emit instruction */
                    for (i,reg) in instr.regs.iter().enumerate() {
                        if out.contains(&i) {
                            let kv = if let Some(ref inputs) = inputs {
                                SavedValue::Known(KnownValue {
                                    itype: instr.itype.clone(),
                                    position: i,
                                    line: rep_line,
                                    inputs: inputs.clone()
                                })
                            } else {
                                SavedValue::UnknownValue(UnknownValue {
                                    line: line,
                                    position: i
                                })
                            };
                            map.insert(reg,&saved.lookup(&kv));
                        }
                    }
                    context.add(instr.clone());
                }
            }
        }
        
    }
    context.phase_finished();
    Ok(())
}
