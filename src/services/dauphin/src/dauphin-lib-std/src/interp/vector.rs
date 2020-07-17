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
use dauphin_interp::types::{ vector_update_poly, append_data };
use serde_cbor::Value as CborValue;

pub struct VectorCopyShallowInterpCommand(Register,Register,Register);

impl InterpCommand for VectorCopyShallowInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let rightval = registers.get(&self.1);
        let rightval = rightval.borrow_mut().get_shared()?;
        let filter = registers.get_indexes(&self.2)?;
        let leftval = registers.get(&self.0);
        let leftval = leftval.borrow_mut().get_exclusive()?;
        let leftval = vector_update_poly(leftval,&rightval,&filter)?;
        registers.write(&self.0,leftval);
        Ok(())    
    }
}

pub struct VectorCopyShallowDeserializer();

impl CommandDeserializer for VectorCopyShallowDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((9,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(VectorCopyShallowInterpCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?)))
    }
}

pub struct VectorAppendDeserializer();

impl CommandDeserializer for VectorAppendDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((10,3))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(VectorAppendInterpCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?)))
    }
}

pub struct VectorAppendInterpCommand(Register,Register,Register);

impl InterpCommand for VectorAppendInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let rightval = registers.get(&self.1);
        let rightval = rightval.borrow_mut().get_shared()?;
        let filter = registers.len(&self.2)?;
        let leftval = registers.get(&self.0);
        let leftval = leftval.borrow_mut().get_exclusive()?;
        let leftdata = append_data(leftval,&rightval,filter)?.0;
        registers.write(&self.0,leftdata);
        Ok(())    
    }
}


pub struct VectorAppendIndexesDeserializer();

impl CommandDeserializer for VectorAppendIndexesDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((17,5))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(VectorAppendIndexesInterpCommand(
            Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?,
            Register::deserialize(&value[3])?,Register::deserialize(&value[4])?)))
    }
}

pub struct VectorAppendIndexesInterpCommand(Register,Register,Register,Register,Register);

impl InterpCommand for VectorAppendIndexesInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let copies = registers.len(&self.4)?;
        if copies == 0 { return Ok(()) }
        let rightval = registers.get_indexes(&self.1)?;
        let start = registers.get_indexes(&self.2)?[0];
        let stride = registers.get_indexes(&self.3)?[0];
        let mut leftval = registers.take_indexes(&self.0)?;
        if start == 0 && stride == 0 {
            for _ in 0..copies {
                leftval.append(&mut rightval.to_vec());
            }
        } else {
            let mut delta = start;
            for _ in 0..copies {
                let mut rightval = rightval.to_vec();
                for v in &mut rightval {
                    *v += delta;
                }
                delta += stride;
                leftval.append(&mut rightval);
            }
        }
        registers.write(&self.0,InterpValue::Indexes(leftval));
        Ok(())
    }
}

pub struct VectorUpdateIndexesDeserializer();

impl CommandDeserializer for VectorUpdateIndexesDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((18,5))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(VectorUpdateIndexesInterpCommand(
            Register::deserialize(&value[0])?,Register::deserialize(&value[1])?,Register::deserialize(&value[2])?,
            Register::deserialize(&value[3])?,Register::deserialize(&value[4])?)))
    }
}


pub struct VectorUpdateIndexesInterpCommand(Register,Register,Register,Register,Register);

impl InterpCommand for VectorUpdateIndexesInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let rightval = registers.get_indexes(&self.1)?;
        let filter = registers.get_indexes(&self.2)?;
        let start = registers.get_indexes(&self.3)?[0];
        let stride = registers.get_indexes(&self.4)?[0];
        let mut leftval = registers.take_indexes(&self.0)?;
        let mut src_it = rightval.iter().cycle();
        if start == 0 && stride == 0 {
            for filter_pos in filter.iter() {
                leftval[*filter_pos] = *src_it.next().unwrap();
            }        
        } else {
            let mut offset = start;
            for filter_pos in filter.iter() {
                leftval[*filter_pos] = *src_it.next().unwrap() + offset;
                offset += stride;
            }
        }
        registers.write(&self.0,InterpValue::Indexes(leftval));
        Ok(())    
    }
}

pub(super) fn library_vector_commands_interp(set: &mut InterpLibRegister) -> Result<(),String> {
    set.push(VectorCopyShallowDeserializer());
    set.push(VectorAppendDeserializer());
    set.push(VectorAppendIndexesDeserializer());
    set.push(VectorUpdateIndexesDeserializer());
    Ok(())
}

