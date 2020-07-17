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

use crate::util::cbor::{ cbor_int, cbor_string };
use crate::command::{ InterpLibRegister, InterpCommand, CommandDeserializer };
use crate::runtime::{ InterpValue, InterpContext, Register };
use serde_cbor::Value as CborValue;

// XXX factor
macro_rules! force_branch {
    ($value:expr,$ty:ident,$branch:ident) => {
        if let $ty::$branch(v) = $value {
            Ok(v)
        } else {
            Err("Cannot extract".to_string())
        }?
    };
}

pub struct NumberConstDeserializer();

impl CommandDeserializer for NumberConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((0,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(NumberConstInterpCommand(Register::deserialize(&value[0])?,*force_branch!(value[1],CborValue,Float)))) 
    }
}

pub struct NumberConstInterpCommand(Register,f64);

impl InterpCommand for NumberConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Numbers(vec![self.1]));
        Ok(())
    }
}

pub struct ConstDeserializer();

impl CommandDeserializer for ConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((1,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let v = force_branch!(&value[1],CborValue,Array);
        let v = v.iter().map(|x| { Ok(*force_branch!(x,CborValue,Integer) as usize) }).collect::<Result<Vec<usize>,String>>()?;
        Ok(Box::new(ConstInterpCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct ConstInterpCommand(Register,Vec<usize>);

impl InterpCommand for ConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Indexes(self.1.to_vec()));
        Ok(())
    }
}

pub struct BooleanConstDeserializer();

impl CommandDeserializer for BooleanConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((2,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(BooleanConstInterpCommand(Register::deserialize(&value[0])?,*force_branch!(value[1],CborValue,Bool))))
    }
}

pub struct BooleanConstInterpCommand(Register,bool);

impl InterpCommand for BooleanConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Boolean(vec![self.1]));
        Ok(())
    }
}

pub struct StringConstDeserializer();

impl CommandDeserializer for StringConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((3,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let v = force_branch!(value[1],CborValue,Text).to_string();
        Ok(Box::new(StringConstInterpCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct StringConstInterpCommand(Register,String);

impl InterpCommand for StringConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Strings(vec![self.1.to_string()]));
        Ok(())
    }
}


pub struct BytesConstDeserializer();

impl CommandDeserializer for BytesConstDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((4,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        let v = force_branch!(value[1],CborValue,Bytes).to_vec();
        Ok(Box::new(BytesConstInterpCommand(Register::deserialize(&value[0])?,v)))
    }
}

pub struct BytesConstInterpCommand(Register,Vec<u8>);

impl InterpCommand for BytesConstInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.registers_mut().write(&self.0,InterpValue::Bytes(vec![self.1.to_vec()]));
        Ok(())
    }
}

pub struct LineNumberDeserializer();

impl CommandDeserializer for LineNumberDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((17,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(LineNumberInterpCommand(cbor_string(&value[0])?,cbor_int(&value[1],None)? as u32)))
    }
}


pub struct LineNumberInterpCommand(String,u32);

impl InterpCommand for LineNumberInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        context.set_line_number(&self.0,self.1);
        Ok(())
    }
}

pub(super) fn const_commands_interp(set: &mut InterpLibRegister) -> Result<(),String> {
    set.push(NumberConstDeserializer());
    set.push(ConstDeserializer());
    set.push(BooleanConstDeserializer());
    set.push(StringConstDeserializer());
    set.push(BytesConstDeserializer());
    set.push(LineNumberDeserializer());
    Ok(())
}
