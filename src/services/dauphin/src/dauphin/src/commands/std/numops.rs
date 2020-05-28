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

use crate::interp::InterpValue;
use crate::model::Register;
use crate::interp::{ Command, CommandSchema, CommandType, CommandTrigger, CommandSet, InterpContext };
use crate::generate::Instruction;
use serde_cbor::Value as CborValue;
use super::library::std;

#[derive(Copy,Clone)]
pub enum InterpNumModOp {
    Incr
}

impl InterpNumModOp {
    fn evaluate(&self, a: &mut f64, b: f64) {
        match self {
            InterpNumModOp::Incr => *a += b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpNumModOp::Incr => "incr",
        }
    }
}

#[derive(Copy,Clone)]
pub enum InterpBinNumOp {
    Plus
}

impl InterpBinNumOp {
    fn evaluate(&self, a: f64, b: f64) -> f64 {
        match self {
            InterpBinNumOp::Plus => a + b
        }
    }

    fn name(&self) -> &str {
        match self {
            InterpBinNumOp::Plus => "plus",
        }
    }
}

#[derive(Copy,Clone)]
pub enum InterpBinBoolOp {
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
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinBoolCommand(self.0,it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
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

pub struct InterpBinNumCommandType(InterpBinNumOp);

impl CommandType for InterpBinNumCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 3,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(self.0,it.regs[0],it.regs[1],it.regs[2])))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpBinNumCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }
}

pub struct InterpBinNumCommand(pub(crate) InterpBinNumOp, pub(crate) Register,pub(crate) Register,pub(crate) Register);

impl Command for InterpBinNumCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let a = registers.get_numbers(&self.2)?;
        let b = &registers.get_numbers(&self.3)?;
        let mut c = vec![];
        let b_len = b.len();
        for (i,a_val) in a.iter().enumerate() {
            c.push(self.0.evaluate(*a_val,b[i%b_len]));
        }
        registers.write(&self.1,InterpValue::Numbers(c));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.1.serialize(),self.2.serialize(),self.3.serialize()])
    }
}

pub struct InterpNumModCommandType(InterpNumModOp);

impl CommandType for InterpNumModCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            values: 2,
            trigger: CommandTrigger::Command(std(self.0.name()))
        }
    }

    fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpNumModCommand(self.0,it.regs[0],it.regs[1])))
    }
    
    fn deserialize(&self, value: &[&CborValue]) -> Result<Box<dyn Command>,String> {
        Ok(Box::new(InterpNumModCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?)))
    }
}

pub struct InterpNumModCommand(pub(crate) InterpNumModOp, pub(crate) Register,pub(crate) Register);

impl Command for InterpNumModCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers();
        let b = &registers.get_numbers(&self.2)?;
        let mut a = registers.take_numbers(&self.1)?;
        let b_len = b.len();
        for (i,a_val) in a.iter_mut().enumerate() {
            self.0.evaluate(a_val,b[i%b_len]);
        }
        registers.write(&self.1,InterpValue::Numbers(a));
        Ok(())
    }

    fn serialize(&self) -> Result<Vec<CborValue>,String> {
        Ok(vec![self.1.serialize(),self.2.serialize()])
    }
}


pub(super) fn library_numops_commands(set: &mut CommandSet) -> Result<(),String> {
    set.push("lt",5,InterpBinBoolCommandType(InterpBinBoolOp::Lt))?;
    set.push("lteq",6,InterpBinBoolCommandType(InterpBinBoolOp::LtEq))?;
    set.push("gt",7,InterpBinBoolCommandType(InterpBinBoolOp::Gt))?;
    set.push("gteq",8,InterpBinBoolCommandType(InterpBinBoolOp::GtEq))?;
    set.push("incr",11,InterpNumModCommandType(InterpNumModOp::Incr))?;
    set.push("plus",12,InterpBinNumCommandType(InterpBinNumOp::Plus))?;
    Ok(())
}
