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

use std::collections::{ HashSet, HashMap };
use super::gencontext::GenContext;
use crate::generate::instruction::{ InstructionType, Instruction };
use crate::model::Register;

/* Relabel instead of copying from sources which are never reused. Recurse this until no change */
pub fn reuse_dead_once(context: &mut GenContext) -> bool {
    let mut progress = false;
    let mut seen_register = HashSet::<Register>::new();
    let mut endoflife_copies = Vec::new();
    let mut rev_instrs = context.get_instructions();
    rev_instrs.reverse();
    /* Flag copies where source is last mention of a variable with appropriate rewrite */
    for instr in rev_instrs {
        let mut endoflife_copy = None;
        if let InstructionType::Copy = instr.itype {
            if !seen_register.contains(&instr.regs[1]) {
                endoflife_copy = Some((instr.regs[0],instr.regs[1]));
            }
        }
        endoflife_copies.push(endoflife_copy);
        seen_register.extend(instr.regs.iter());
    }
    endoflife_copies.reverse();
    /* Rewrite sources after end-of-life copy comes up */
    let mut rewrite_rules = HashMap::new();
    for (i,instr) in context.get_instructions().iter().enumerate() {
        if let Some((dst,src)) = endoflife_copies[i] {
            rewrite_rules.insert(dst,src);
            progress = true;
        } else {
            let new_regs = instr.regs.iter().map(|r| *rewrite_rules.get(r).unwrap_or(r)).collect::<Vec<_>>();
            context.add(Instruction::new(instr.itype.clone(),new_regs));
        }
    }
    context.phase_finished();
    progress
}

pub fn reuse_dead(context: &mut GenContext) {
    while reuse_dead_once(context) {}
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::interp::mini_interp;
    use super::super::linearize;
    use super::super::remove_aliases;
    use super::super::prune;
    use super::super::copy_on_write;

    #[test]
    fn reuse_dead_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts,true).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        prune(&mut context);
        copy_on_write(&mut context);
        prune(&mut context);
        print!("{:?}\n",context);
        reuse_dead(&mut context);
        print!("{:?}\n",context);
        let (_,strings) = mini_interp(&mut context).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}