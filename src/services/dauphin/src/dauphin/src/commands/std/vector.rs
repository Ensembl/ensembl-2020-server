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

use crate::model::{ Register, RegisterSignature, cbor_make_map };
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext, PreImageOutcome, PreImagePrepare, TimeTrialCommandType, TimeTrial, regress, trial_write, trial_signature };
use crate::generate::{ Instruction, InstructionType, PreImageContext };
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_array, cbor_bool, cbor_map };
use crate::typeinf::MemberMode;
use super::super::common::vectorcopy::{ vector_update_offsets, vector_update_lengths, vector_update_poly, vector_push, vector_register_copy, vector_append, append_data };
use super::super::common::vectorsource::RegisterVectorSource;
use super::super::common::sharedvec::{ SharedVec };
use super::super::common::writevec::WriteVec;
use super::extend::ExtendCommandType;
use super::library::std;
use crate::cli::Config;
use crate::interp::CompilerLink;

struct VectorCopyShallowTimeTrial();

impl TimeTrialCommandType for VectorCopyShallowTimeTrial {
    fn timetrial_make_trials(&self) -> (i64,i64) { (1,10) }

    fn local_prepare(&self, context: &mut InterpContext, t: i64) {
        let t = t as usize;
        trial_write(context,0,t*100,|x| x);
        trial_write(context,1,t*100,|x| x);
        trial_write(context,2,t*100,|x| x);
        context.registers().commit();
    }

    fn timetrial_make_command(&self, _: i64, _linker: &CompilerLink, _config: &Config) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorCopyShallow(Register(0),Register(1),Register(2),None)))
    }
}

pub struct VectorCopyShallowType(Option<TimeTrial>);

impl VectorCopyShallowType {
    fn new() -> VectorCopyShallowType { VectorCopyShallowType(None) }
}

impl CommandType for VectorCopyShallowType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std("_vector_copy_shallow"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorCopyShallow(it.regs[0].clone(),it.regs[1].clone(),it.regs[2].clone(),self.0.clone())))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(VectorCopyShallow(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?,self.0.clone())))
    }

    fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
        let timings = TimeTrial::run(&VectorCopyShallowTimeTrial(),linker,config)?;
        Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
    }

    fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
        let t = cbor_map(value,&vec!["t"])?;
        self.0 = Some(TimeTrial::deserialize(&t[0])?);
        Ok(())
    }
}

pub struct VectorCopyShallow(Register,Register,Register,Option<TimeTrial>);

impl VectorCopyShallow {
    fn size(&self, context: &PreImageContext) -> Option<usize> {
        let unit = if let Some(size) = context.get_reg_size(&self.1) {
            size
        } else {
            return None
        };
        let copies = if let Some(size) = context.get_reg_size(&self.2) {
            size
        } else {
            return None
        };
        Some(unit*copies)
    }
}

impl Command for VectorCopyShallow {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let rightval = registers.get(&self.1);
        let rightval = rightval.borrow_mut().get_shared()?;
        let filter = registers.get_indexes(&self.2)?;
        let leftval = registers.get(&self.0);
        let leftval = leftval.borrow_mut().get_exclusive()?;
        let leftval = vector_update_poly(leftval,&rightval,&filter)?;
        registers.write(&self.0,leftval);
        Ok(())    
    }

    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize(),self.1.serialize(),self.2.serialize()]))
    }

    fn simple_preimage(&self, context: &mut PreImageContext) -> Result<PreImagePrepare,String> { 
        Ok(if context.is_reg_valid(&self.1) && context.is_reg_valid(&self.2) {
            PreImagePrepare::Replace
        } else if let Some(size) = self.size(context) {
            PreImagePrepare::Keep(vec![(self.0.clone(),size)])
        } else {
            PreImagePrepare::Keep(vec![])
        })
    }

    fn execution_time(&self, context: &PreImageContext) -> f64 {
        if let Some(size) = self.size(context) {
            self.3.as_ref().map(|x| x.evaluate(size as f64/100.)).unwrap_or(1.)
        } else {
            1.
        }
    }
}

pub(super) fn library_vector_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("_vector_copy_shallow",15,VectorCopyShallowType::new())?;
    Ok(())
}
