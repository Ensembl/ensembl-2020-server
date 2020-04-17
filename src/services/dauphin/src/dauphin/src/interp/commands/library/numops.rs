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

use crate::interp::context::{InterpContext };
use crate::interp::InterpValue;
use crate::model::Register;
use crate::interp::commandsets::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet };
use crate::generate::Instruction;
use serde_cbor::Value as CborValue;

#[derive(Copy,Clone)]
pub(crate) enum InterpBinBoolOp {
    Lt,
    LtEq,
    Gt,
    GtEq
}

impl InterpBinBoolOp {
    fn evaluate(&self, a: f64, b: f64) -> bool {
        match self {
            InterpBinBoolOp::Lt => a < b,
            InterpBinBoolOp::LtEq => a <= b,
            InterpBinBoolOp::Gt => a > b,
            InterpBinBoolOp::GtEq => a >= b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpBinBoolOp::Lt => "lt",
            InterpBinBoolOp::LtEq => "lteq",
            InterpBinBoolOp::Gt => "gt",
            InterpBinBoolOp::GtEq => "gteq"
        }
    }
}

pub struct InterpBinBoolCommandType(InterpBinBoolOp);

impl CommandType for InterpBinBoolCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(self.0.name().to_string())
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }
}

pub struct InterpBinBoolCommand(pub(crate) InterpBinBoolOp, pub(crate) Register,pub(crate) Register,pub(crate) Register);

impl Command for InterpBinBoolCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = registers.get_numbers(&self.2)?;
        let b = &registers.get_numbers(&self.3)?;
        let mut c = vec![];
        let b_len = b.len();
        for (i,a_val) in a.iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        registers.write(&self.1,InterpValue::Boolean(c));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()])
    }
}

pub(super) fn library_numops_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("lt",5,InterpBinBoolCommandType(InterpBinBoolOp::Lt))?;
    set.push("lteq",6,InterpBinBoolCommandType(InterpBinBoolOp::LtEq))?;
    set.push("gt",7,InterpBinBoolCommandType(InterpBinBoolOp::Gt))?;
    set.push("gteq",8,InterpBinBoolCommandType(InterpBinBoolOp::GtEq))?;
    Ok(())
}
