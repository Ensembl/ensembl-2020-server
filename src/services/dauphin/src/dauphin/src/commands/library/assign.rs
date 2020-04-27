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

use crate::model::{ Register, RegisterSignature };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext };
use crate::generate::{ Instruction, InstructionType };
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_array, cbor_bool };
use crate::typeinf::MemberMode;
use super::super::common::vectorcopy::VectorCopy;
use super::super::common::vectorsource::RegisterVectorSource;

fn assign_unfiltered(context: &mut InterpContext, regs: &Vec<Register>) -> Result<(),String> {
    let registers = context.registers();
    let n = regs.len()/2;
    for i in 0..n {
        registers.copy(&regs[i],&regs[i+n])?;
    }
    Ok(())
}

/// XXX ban multi-Lvalue
fn assign_filtered(context: &mut InterpContext, sig: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    let filter_reg = context.registers().get_indexes(&regs[0])?;
    let mut vector_copies = vec![];
    for (vr1,vr2) in sig[1].iter().zip(sig[2].iter()) {
        let vrs1 = RegisterVectorSource::new(&regs);
        let vrs2 = RegisterVectorSource::new(&regs);
        vector_copies.push(VectorCopy::new(context,vrs1,vr1.1,vrs2,vr2.1,&filter_reg)?);
    }
    for vc in vector_copies {
        vc.copy(context)?;
    }
    Ok(())
}

fn assign(context: &mut InterpContext, filtered: bool, purposes: &RegisterSignature, regs: &Vec<Register>) -> Result<(),String> {
    if filtered {
        assign_filtered(context,purposes,regs)?;
    } else {
        assign_unfiltered(context,regs)?;
    }
    Ok(())
}

pub struct AssignCommandType();

impl CommandType for AssignCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command("assign".to_string())
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(AssignCommand(sig[0].get_mode() != MemberMode::LValue,sig.clone(),it.regs.to_vec())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        let regs = cbor_array(&value[2],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        let sig = RegisterSignature::deserialize(&value[1],false)?;
        Ok(Box::new(AssignCommand(cbor_bool(&value[0])?,sig,regs)))
    }
}

pub struct AssignCommand(pub(crate) bool, pub(crate) RegisterSignature, pub(crate) Vec<Register>);

impl Command for AssignCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        assign(context,self.0,&self.1,&self.2)?;
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        let regs = CborValue::Array(self.2.iter().map(|x| x.serialize()).collect());
        Ok(vec![CborValue::Bool(self.0),self.1.serialize(false)?,regs])
    }
}

pub(super) fn library_assign_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("assign",9,AssignCommandType())?;
    Ok(())
}