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

use std::cell::RefCell;
use std::rc::Rc;
use dauphin_interp::command::{ CommandDeserializer, InterpCommand, Identifier };
use dauphin_interp::runtime::{ InterpContext };
use dauphin_compile::command::{ Instruction, Command, CommandSchema, CommandTrigger, CommandType };
use serde_cbor::Value as CborValue;

pub struct FakeInterpCommand(Rc<RefCell<u32>>,u32);

impl InterpCommand for FakeInterpCommand {
    fn execute(&self, context: &mut InterpContext) -> Result<(),String> {
        *self.0.borrow_mut() = self.1;
        Ok(())
    }
}

pub struct FakeDeserializer(pub Rc<RefCell<u32>>,pub u32);

impl CommandDeserializer for FakeDeserializer {
    fn get_opcode_len(&self) -> Result<Option<(u32,usize)>,String> { Ok(Some((self.1,0))) }
    fn deserialize(&self, opcode: u32, value: &[&CborValue]) -> Result<Box<dyn InterpCommand>,String> {
        Ok(Box::new(FakeInterpCommand(self.0.clone(),self.1)))
    }
}

pub fn fake_trigger(name: &str) -> CommandTrigger {
    CommandTrigger::Command(Identifier::new("fake",name))
}

struct FakeCommandType(String);

impl CommandType for FakeCommandType {
    fn get_schema(&self) -> CommandSchema {
        CommandSchema {
            trigger: fake_trigger(&self.0),
            values: 0
        }
    }

    fn from_instruction(&self, _it: &Instruction) -> Result<Box<dyn Command>,String> {
        Err(format!("unable"))
    }
}

pub fn fake_command(name: &str) -> impl CommandType {
    FakeCommandType(name.to_string())
}
