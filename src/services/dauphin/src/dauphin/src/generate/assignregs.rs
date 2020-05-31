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
use crate::model::Register;
use super::gencontext::GenContext;
use super::instruction::Instruction;

fn find_first_last_use(context: &mut GenContext) -> HashMap<Register,(usize,usize)> {
    /* find first and last use of every register */
    let mut out = HashMap::new();
    let mut seen = HashSet::new();
    for (i,instr) in context.get_instructions().iter().enumerate() {
        for reg in instr.regs.iter() {
            if !seen.contains(reg) {
                out.insert(*reg,(i,0));
                seen.insert(reg);
            }
        }
    }
    let mut seen = HashSet::new();
    let mut rev_instrs = context.get_instructions();
    rev_instrs.reverse();
    for (i,instr) in rev_instrs.iter().enumerate() {
        for reg in instr.regs.iter() {
            if !seen.contains(reg) {
                out.get_mut(reg).unwrap().1 = rev_instrs.len()-i-1;
                seen.insert(reg);
            }
        }
    }
    out
}

fn allocate(regs: Vec<Register>, reg_ranges: HashMap<Register,(usize,usize)>) -> HashMap<Register,Register> {
    let mut allocation = HashMap::new();
    let mut in_use = Vec::new();
    for reg in &regs {
        let mut overlap : HashSet<usize> = HashSet::new();
        let (first,last) = reg_ranges.get(reg).unwrap();
        while in_use.len() <= *last {
            in_use.push(HashSet::new());
        }
        for i in *first..(*last+1) {
            overlap.extend(in_use[i].iter());
        }
        let mut reg_num = 1;
        while overlap.contains(&reg_num) {
            reg_num += 1;
        }
        allocation.insert(*reg,Register(reg_num));
        for i in *first..(*last+1) {
            in_use[i].insert(reg_num);
        }
    }
    allocation
}

pub fn assign_regs(context: &mut GenContext) {
    let range = find_first_last_use(context);
    let priorities : HashMap<_,_> = range.iter().map(|(k,v)| (*k,v.1-v.0+1)).collect();
    let mut regs : Vec<_> = priorities.keys().cloned().collect();
    regs.sort_by_key(|k| priorities.get(k).unwrap());
    regs.reverse(); /* longest-lived first */
    let allocation = allocate(regs,range);
    for instr in context.get_instructions().iter() {
        let new_regs = instr.regs.iter().map(|r| *allocation.get(r).unwrap()).collect::<Vec<_>>();
        context.add(Instruction::new(instr.itype.clone(),new_regs));
    }
    context.phase_finished();
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::simplify::simplify;
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_librarysuite_builder };
    use super::super::codegen::generate_code;
    use super::super::call::call;
    use super::super::linearize::linearize;
    use super::super::dealias::remove_aliases;
    use super::super::cow::copy_on_write;
    use super::super::compilerun::compile_run;
    use super::super::reusedead::reuse_dead;
    use super::super::prune::prune;

    // XXX test pruning, eg fewer lines
    #[test]
    fn assign_regs_smoke() {
        let config = xxx_test_config();
        let linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        compile_run(&linker,&resolver,&mut context).expect("m");
        prune(&mut context);
        copy_on_write(&mut context);
        prune(&mut context);
        compile_run(&linker,&resolver,&mut context).expect("n");
        reuse_dead(&mut context);
        assign_regs(&mut context);
        print!("{:?}",context);
        let (_,strings) = mini_interp(&mut context.get_instructions(),&linker,&config).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}