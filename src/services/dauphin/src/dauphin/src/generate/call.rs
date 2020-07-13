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
                let mut rs = RegisterSignature::new();
                let mut flows = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    flows.push(match modes[i] {
                        MemberMode::InOut => MemberDataFlow::InOut,
                        MemberMode::Out => MemberDataFlow::Out,
                        _ => MemberDataFlow::In
                    });
                    rs.add(ComplexRegisters::new(&context.get_defstore(),modes[i],&type_)?);
                }
                context.add(Instruction::new(InstructionType::Call(identifier.clone(),true,rs,flows),instr.regs.to_vec()));
            },
            
            InstructionType::Operator(identifier) => {
                let mut rs = RegisterSignature::new();
                let mut flows = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    flows.push(if i == 0 { MemberDataFlow::Out } else { MemberDataFlow::In });
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    rs.add(ComplexRegisters::new(&context.get_defstore(),if i==0 { MemberMode::Out } else { MemberMode::In },&type_)?);
                }
                context.add(Instruction::new(InstructionType::Call(identifier.clone(),false,rs,flows),instr.regs.to_vec()));
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
    use crate::resolver::common_resolver;
    use crate::parser::{ Parser };
    use crate::generate::generate;
    use crate::interp::{ mini_interp, CompilerLink, xxx_test_config, make_compiler_suite };

    #[test]
    fn module_smoke() {
        let config = xxx_test_config();
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import("search:codegen/module-smoke").expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        mini_interp(&instrs,&mut linker,&config,"main").expect("x");
    }
}