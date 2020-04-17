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
 *  
 *  vscode-fold=1
 */

use crate::generate::{ Instruction, InstructionType };
use crate::typeinf::{ MemberMode, MemberDataFlow };
use super::gencontext::GenContext;
use crate::model::{ RegisterSignature, ComplexRegisters };

pub fn call(context: &mut GenContext) -> Result<(),String> {
    for instr in &context.get_instructions() {
        match &instr.itype {
            InstructionType::Proc(name,modes) => {
                let mut rs = RegisterSignature::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    let flow = match modes[i] {
                        MemberMode::LValue => MemberDataFlow::JustifiesCall,
                        _ => MemberDataFlow::Normal
                    };
                    rs.add(ComplexRegisters::new(&context.get_defstore(),modes[i],&type_,flow)?);
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),true,rs),instr.regs.to_vec()));
            },
            
            InstructionType::Operator(name) => {
                let mut rs = RegisterSignature::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    let mode = if i == 0 { MemberDataFlow::JustifiesCall } else { MemberDataFlow::Normal };
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    rs.add(ComplexRegisters::new(&context.get_defstore(),MemberMode::RValue,&type_,mode)?);
                }
                context.add(Instruction::new(InstructionType::Call(name.to_string(),false,rs),instr.regs.to_vec()));
            },

            _ => { context.add(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}
