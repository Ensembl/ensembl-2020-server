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

use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::gencontext::GenContext;
use crate::model::{ RegisterSignature, ComplexRegisters };

pub fn call(context: &mut GenContext) -> Result<(),String> {
    for instr in &context.get_instructions() {
        match &instr.itype {
            InstructionType::Proc(identifier,modes) => {
                print!("mfind {:?}\n",identifier);
                let mut rs = RegisterSignature::new();
                let mut flows = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    flows.push(match modes[i] {
                        MemberMode::LValue => MemberDataFlow::JustifiesCall,
                        _ => MemberDataFlow::Normal
                    });                    
                    rs.add(ComplexRegisters::new(&context.get_defstore(),modes[i],&type_)?);
                }
                context.add(Instruction::new(InstructionType::Call(identifier.1.to_string(),true,rs,flows),instr.regs.to_vec()));
            },
            
            InstructionType::Operator(identifier) => {
                print!("mfind {:?}\n",identifier);
                let mut rs = RegisterSignature::new();
                let mut flows = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    flows.push(if i == 0 { MemberDataFlow::JustifiesCall } else { MemberDataFlow::Normal });
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    rs.add(ComplexRegisters::new(&context.get_defstore(),MemberMode::RValue,&type_)?);
                }
                context.add(Instruction::new(InstructionType::Call(identifier.1.to_string(),false,rs,flows),instr.regs.to_vec()));
            },

            _ => { context.add(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::lexer::Lexer;
    use crate::resolver::test_resolver;
    use crate::parser::{ Parser };
    use crate::generate::{ generate_code, generate };
    use crate::interp::mini_interp;

    #[test]
    fn module_smoke() {
        let resolver = test_resolver();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/module-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts,true).expect("codegen");
        generate(&mut context,&defstore).expect("j");
        mini_interp(&mut context).expect("x");
    }
}