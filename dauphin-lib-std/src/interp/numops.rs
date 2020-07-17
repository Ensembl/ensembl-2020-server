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

use dauphin_interp::command::{ InterpCommand, CommandDeserializer, InterpLibRegister };
use dauphin_interp::runtime::{ InterpContext, InterpValue, Register };
use serde_cbor::Value as CborValue;

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

    pub fn name(&self) -> &str {
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

    pub fn name(&self) -> &str {
        match self {
            InterpBinBoolOp::Lt => "lt",
            InterpBinBoolOp::LtEq => "lteq",
            InterpBinBoolOp::Gt => "gt",
            InterpBinBoolOp::GtEq => "gteq"
        }
    }
}

pub struct BinBoolDeserializer(InterpBinBoolOp,u32);

impl CommandDeserializer for BinBoolDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((self.1,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(InterpBinBoolInterpCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }
}

pub struct InterpBinBoolInterpCommand(InterpBinBoolOp,Register,Register,Register);

impl InterpCommand for InterpBinBoolInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
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
}

pub struct BinNumDeserializer(InterpBinNumOp,u32);

impl CommandDeserializer for BinNumDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((self.1,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(InterpBinNumInterpCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            Register::deserialize(&value[2])?)))
    }
}

pub struct InterpBinNumInterpCommand(InterpBinNumOp,Register,Register,Register);

impl InterpCommand for InterpBinNumInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
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
}


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

    pub fn name(&self) -> &str {
        match self {
            InterpNumModOp::Incr => "incr",
        }
    }
}

pub struct NumModDeserializer(InterpNumModOp,u32);

impl CommandDeserializer for NumModDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((self.1,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let filter = if *value[2] == CborValue::Null { 
            None
        } else {
            Some(Register::deserialize(value[2])?)
        };
        Ok(Box::new(InterpNumModInterpCommand(
            self.0,
            Register::deserialize(&value[0])?,
            Register::deserialize(&value[1])?,
            filter)))
    }
}

pub struct InterpNumModInterpCommand(InterpNumModOp,Register,Register,Option<Register>);

impl InterpNumModInterpCommand {
    fn execute_unfiltered(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let b = &registers.get_numbers(&self.2)?;
        let mut a = registers.take_numbers(&self.1)?;
        let b_len = b.len();
        for (i,a_val) in a.iter_mut().enumerate() {
            self.0.evaluate(a_val,b[i%b_len]);
        }
        registers.write(&self.1,InterpValue::Numbers(a));
        Ok(())
    }

    fn execute_filtered(&self, context: &mut InterpContext) -> Result<(),String> {
        let filter : &[usize] = &context.registers_mut().get_indexes(self.3.as_ref().unwrap())?;
        let registers = context.registers_mut();
        let b = &registers.get_numbers(&self.2)?;
        let mut a = registers.take_numbers(&self.1)?;
        let b_len = b.len();
        for (i,pos) in filter.iter().enumerate() {
            self.0.evaluate(&mut a[*pos],b[i%b_len]);
        }
        registers.write(&self.1,InterpValue::Numbers(a));
        Ok(())
    }
}

impl InterpCommand for InterpNumModInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        if self.3.is_some() {
            self.execute_filtered(context)
        } else {
            self.execute_unfiltered(context)
        }
    }
}

pub(super) fn library_numops_commands_interp(set: &mut InterpLibRegister) -> Result<(),String> {
    set.push(BinBoolDeserializer(InterpBinBoolOp::Lt,5));
    set.push(BinBoolDeserializer(InterpBinBoolOp::LtEq,6));
    set.push(BinBoolDeserializer(InterpBinBoolOp::Gt,7));
    set.push(BinBoolDeserializer(InterpBinBoolOp::GtEq,8));
    set.push(NumModDeserializer(InterpNumModOp::Incr,11));
    set.push(BinNumDeserializer(InterpBinNumOp::Plus,12));
    Ok(())
}
