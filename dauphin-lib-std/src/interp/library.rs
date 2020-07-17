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

use dauphin_interp::command::{ CommandSetId, InterpCommand, CommandDeserializer, InterpLibRegister };
use dauphin_interp::runtime::{ InterpContext, Register };
use dauphin_interp::util::templates::NoopDeserializer;
use serde_cbor::Value as CborValue;
use super::eq::{ library_eq_command_interp };
use super::numops::{ library_numops_commands_interp };
use super::vector::{ library_vector_commands_interp };
use super::print::{ library_print_commands_interp };

pub fn std_id() -> CommandSetId {
    CommandSetId::new("std",(0,0),0x8A07AE1254D6E44B)
}

pub struct AssertDeserializer();

impl CommandDeserializer for AssertDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((4,2))) }
    fn deserialize(&self, _opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(AssertInterpCommand(Register::deserialize(&value[0])?,Register::deserialize(&value[1])?)))
    }
}

pub struct AssertInterpCommand(Register,Register);

impl InterpCommand for AssertInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        let registers = context.registers_mut();
        let a = &registers.get_boolean(&self.0)?;
        let b = &registers.get_boolean(&self.1)?;
        for i in 0..a.len() {
            if a[i] != b[i%b.len()] {
                return Err(format!("assertion failed index={}!",i));
            }
        }
        Ok(())
    }
}

pub fn make_std_interp() -> Result<InterpLibRegister,String> {
    let mut set = InterpLibRegister::new(&std_id());
    library_eq_command_interp(&mut set)?;
    set.push(AssertDeserializer());
    set.push(NoopDeserializer(13));
    library_print_commands_interp(&mut set)?;
    library_numops_commands_interp(&mut set)?;
    library_vector_commands_interp(&mut set)?;
    Ok(set)
}
