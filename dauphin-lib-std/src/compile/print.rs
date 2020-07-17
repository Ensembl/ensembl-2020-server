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

use dauphin_compile::command::{ Command, CommandSchema, CommandType, CommandTrigger, Instruction, InstructionType };
use dauphin_interp::command::Identifier;
use dauphin_interp::runtime::Register;
use dauphin_interp::types::RegisterSignature;
use serde_cbor::Value as CborValue;

pub struct FormatCommandType();

impl CommandType for FormatCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(Identifier::new("std","format"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,sig,_) = &it.itype {
            Ok(Box::new(FormatCommand(it.regs.to_vec(),sig.clone())))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct FormatCommand(Vec<Register>,RegisterSignature);

impl Command for FormatCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        let regs = CborValue::Array(self.0.iter().map(|x| x.serialize()).collect());
        Ok(Some(vec![regs,self.1.serialize(true)?]))
    }
}

pub struct PrintCommandType();

impl CommandType for PrintCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 1,
            trigger: CommandTrigger::Command(Identifier::new("std","print"))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        if let InstructionType::Call(_,_,_sig,_) = &it.itype {
            Ok(Box::new(PrintCommand(it.regs[0])))
        } else {
            Err("unexpected instruction".to_string())
        }
    }    
}

pub struct PrintCommand(Register);

impl Command for PrintCommand {
    fn serialize(&self) -> Result<Option<Vec<CborValue>>,String> {
        Ok(Some(vec![self.0.serialize()]))
    }    
}
