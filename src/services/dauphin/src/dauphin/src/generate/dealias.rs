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
use crate::model::Register;
use super::gencontext::GenContext;
use super::instruction::{ Instruction, InstructionType };

struct Aliases(HashMap<Register,Register>);

impl Aliases {
    fn lookup(&self, alias: &Register) -> Register {
        match self.0.get(&alias) {
            Some(further) => self.lookup(further),
            None => *alias
        }
    }

    fn alias(&mut self, alias: &Register, target: &Register) {
        self.0.insert(*alias,self.lookup(target));
    }
}

pub fn remove_aliases(context: &mut GenContext) {
    let mut aliases = Aliases(HashMap::new());
    for instr in context.get_instructions() {
        match instr.itype {
            InstructionType::Alias => {
                aliases.alias(&instr.regs[0],&instr.regs[1]);
            },
            _ => {
                context.add(Instruction::new(instr.itype,instr.regs.iter().map(|x| aliases.lookup(x)).collect()));
            }
        }
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
    use super::super::codegen::generate_code;
    use super::super::call::call;
    use super::super::linearize::linearize;
    use crate::interp::{ mini_interp, xxx_compiler_link, xxx_test_config };

    #[test]
    fn dealias_smoke() {
        // XXX check all aliases gone
        let config = xxx_test_config();
        let resolver = common_resolver(&config).expect("a");
        let mut lexer = Lexer::new(&resolver);
        lexer.import("search:codegen/linearize-refsquare").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,&stmts,true).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize(&mut context).expect("linearize");
        print!("BEFORE {:?}\n",context);
        remove_aliases(&mut context);
        print!("AFTER {:?}\n",context);
        let linker = xxx_compiler_link().expect("y");

        let (values,strings) = mini_interp(&mut context.get_instructions(),&linker,&config).expect("x");
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        for instr in context.get_instructions() {
            if let InstructionType::Alias = instr.itype {
                assert!(false);
            }
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

}