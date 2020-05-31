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
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, xxx_compiler_link, xxx_test_config };

    #[test]
    fn reuse_dead_smoke() {
        let config = xxx_test_config();
        let resolver = common_resolver(&config).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let linker = xxx_compiler_link().expect("y");
        let config = xxx_test_config();
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        print!("{:?}",instrs.iter().map(|x| format!("{:?}",x)).collect::<Vec<_>>().join(""));
        let (_,strings) = mini_interp(&instrs,&linker,&config).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}