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

use dauphin_compile::command::{
    Command, CommandSchema, CommandType, CommandTrigger, PreImageOutcome, Instruction, InstructionType, TimeTrial
};
use dauphin_compile::model::PreImageContext;
use dauphin_interp::command::InterpCommand;
use dauphin_interp::runtime::Register;
use dauphin_interp::types::RegisterSignature;
use dauphin_compile::util::{ vector_push_instrs, vector_append, vector_append_offsets, vector_register_copy_instrs, vector_append_lengths };
use serde_cbor::Value as CborValue;
use super::library::std;

fn extend(context: &mut PreImageContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<Vec<Instruction>,String> {
    let mut out = vec![];
    for (vr_z,vr_a) in sig[0].iter().zip(sig[1].iter()) {
        out.append(&mut vector_register_copy_instrs(&vr_z.1,&vr_a.1,regs)?);
    }
    let one = context.new_register();
    out.push(Instruction::new(InstructionType::Const(vec![1]),vec![one]));
    let zero = context.new_register();
    out.push(Instruction::new(InstructionType::Const(vec![0]),vec![zero]));
    for (z,b) in sig[0].iter().zip(sig[2].iter()) {
        let depth = z.1.depth();
        if depth > 0 {
            /* get start of penultimate layer for post push */
            let start = context.new_register();
            let reg_off = if depth > 1 {
                z.1.offset_pos(depth-2)?
            } else {
                z.1.data_pos()
            };
            out.push(Instruction::new(InstructionType::Length,vec![start,regs[reg_off]]));
            /* push all but top layer */
            out.append(&mut vector_push_instrs(context,z.1,b.1,&start,regs)?);
            /* push top layer */
            out.push(vector_append_offsets(z.1,b.1,&start,&zero,&one,regs,depth-1)?);
            out.push(vector_append_lengths(z.1,b.1,&zero,&one,&regs,depth-1)?);
        } else {
            out.push(vector_append(z.1,b.1,&one,&regs)?);
        }
    }
    Ok(out)
}

pub struct ExtendCommandType(Option<TimeTrial>);

impl ExtendCommandType {
    pub fn new() -> ExtendCommandType { ExtendCommandType(None) }
}

impl CommandType for ExtendCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 0,
            trigger: CommandTrigger::Command(std("extend"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(ExtendCommand(sig.clone(),it.regs.to_vec(),self.0.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct ExtendCommand(RegisterSignature,Vec<Register>,Option<TimeTrial>);

impl Command for ExtendCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(None)
    }

    fn preimage(&self, context: &mut PreImageContext, _ic: Option<Box<dyn InterpCommand>>) -> Result<PreImageOutcome,String> {
        Ok(PreImageOutcome::Replace(extend(context,&self.0,&self.1)?))
    }
}
