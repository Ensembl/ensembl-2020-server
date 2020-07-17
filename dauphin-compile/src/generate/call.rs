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

use crate::command::{ Instruction, InstructionType };
use super::gencontext::GenContext;
use dauphin_interp::types::{ RegisterSignature, MemberMode, MemberDataFlow };
use crate::model::{ make_full_type };

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
                    rs.add(make_full_type(&context.get_defstore(),modes[i],&type_)?);
                }
                context.add(Instruction::new(InstructionType::Call(identifier.clone(),true,rs,flows),instr.regs.to_vec()));
            },
            
            InstructionType::Operator(identifier) => {
                let mut rs = RegisterSignature::new();
                let mut flows = Vec::new();
                for (i,reg) in instr.regs.iter().enumerate() {
                    flows.push(if i == 0 { MemberDataFlow::Out } else { MemberDataFlow::In });
                    let type_ = context.xxx_types().get(&reg).unwrap().clone();
                    rs.add(make_full_type(&context.get_defstore(),if i==0 { MemberMode::Out } else { MemberMode::In },&type_)?);
                }
                context.add(Instruction::new(InstructionType::Call(identifier.clone(),false,rs,flows),instr.regs.to_vec()));
            },

            _ => { context.add(instr.clone()); }
        }
    }
    context.phase_finished();
    Ok(())
}
