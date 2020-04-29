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

use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger };
use crate::model::Register;
use crate::generate::{ Instruction, InstructionSuperType };
use serde_cbor::Value as CborValue;

pub struct BuiltinCommandType {
    supertype: InstructionSuperType,
    values: usize,
    ctor: Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>
}

impl BuiltinCommandType {
    pub fn new(supertype: InstructionSuperType, values: usize, ctor: Box<dyn Fn(&[Register]) -> Result<Box<dyn Command>,String>>) -> BuiltinCommandType {
        BuiltinCommandType {
            supertype, values, ctor
        }
    }
}

impl CommandType for BuiltinCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: self.values,
            trigger: CommandTrigger::Instruction(self.supertype)
        }
    }
    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        (self.ctor)(&it.regs)
    }

    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        (self.ctor)(&(0..self.values).map(|x| Register::deserialize(value[x])).collect::<Result<Vec<_>,String>>()?)
    }
}
