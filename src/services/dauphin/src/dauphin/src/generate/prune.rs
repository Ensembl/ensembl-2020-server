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

pub fn prune(context: &mut GenContext) {
    let mut justified_calls = Vec::new();
    let mut justified_regs = HashSet::new();
    let mut rev_instrs = context.get_instructions();
    rev_instrs.reverse();
    for instr in rev_instrs {
        let mut call_justified = false;
        if instr.itype.self_justifying_call() {
            call_justified = true;
        }
        for idx in instr.itype.out_registers() {
            if justified_regs.contains(&instr.regs[idx]) {
                call_justified = true;
                break;
            }
        }
        justified_calls.push(call_justified);
        if call_justified {
            let (regs,itype) = (instr.regs,instr.itype);
            for (i,reg) in regs.iter().enumerate() {
                if itype.out_only_registers().contains(&i) {
                    justified_regs.remove(reg);
                } else {
                    justified_regs.insert(*reg);
                }
            }
        }
    }
    justified_calls.reverse();
    for (i,instr) in context.get_instructions().iter().enumerate() {
        if justified_calls[i] {
            context.add(instr.clone());
        }
    }
    context.phase_finished();
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_compiler_suite };

    // XXX test pruning, eg fewer lines
    #[test]
    fn prune_smoke() {
        let mut config = xxx_test_config();
        config.set_opt_seq("p");
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        print!("{:?}",instrs.iter().map(|x| format!("{:?}",x)).collect::<Vec<_>>().join(""));
        let (_values,strings) = mini_interp(&instrs,&mut linker,&config,"main").expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0], [2], [0], [4]]", "[[0], [2], [9, 9, 9], [9, 9, 9]]", "[0, 0, 0]", "[[0], [2], [8, 9, 9], [9, 9, 9]]"],strings);
    }
}