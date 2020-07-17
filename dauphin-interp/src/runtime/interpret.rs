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

use std::slice::Iter;
use crate::command::{ InterpCommand, InterpreterLink };
use crate::runtime::{ Register, InterpContext };

pub trait InterpretInstance<'a> {
    fn finish(&mut self) -> InterpContext;
    fn more(&mut self) -> Result<bool,String>;
}

pub struct StandardInterpretInstance<'a> {
    commands: Iter<'a,Box<dyn InterpCommand>>,
    context: Option<InterpContext>
}

impl<'a> StandardInterpretInstance<'a> {
    pub fn new(interpret_linker: &'a InterpreterLink, name: &str) -> Result<StandardInterpretInstance<'a>,String> {
        let context = interpret_linker.new_context();
        Ok(StandardInterpretInstance {
            commands: interpret_linker.get_commands(name)?.iter(),
            context: Some(context)
        })
    }

    fn more_internal(&mut self) -> Result<bool,String> {
        let context = self.context.as_mut().unwrap();
        while let Some(command) = self.commands.next() {
            command.execute(context)?;
            context.registers_mut().commit();
            if context.test_pause() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn error_message(&self, msg: String) -> String {
        let context = self.context.as_ref().unwrap();
        let line = context.get_line_number();
        if line.1 != 0 {
            format!("{} at {}:{}",msg,line.0,line.1)
        } else {
            msg
        }
    }
}

impl<'a> InterpretInstance<'a> for StandardInterpretInstance<'a> {
    fn more(&mut self) -> Result<bool,String> {
        self.more_internal().map_err(|msg| self.error_message(msg))
    }

    fn finish(&mut self) -> InterpContext { self.context.take().unwrap() }
}

pub struct DebugInterpretInstance<'a> {
    commands: Iter<'a,Box<dyn InterpCommand>>,
    context: Option<InterpContext>,
    instrs: Vec<(String,Vec<Register>)>,
    index: usize
}

impl<'a> DebugInterpretInstance<'a> {
    pub fn new(interpret_linker: &'a InterpreterLink, instrs: &[(String,Vec<Register>)], name: &str) -> Result<DebugInterpretInstance<'a>,String> {
        let context = interpret_linker.new_context();
        Ok(DebugInterpretInstance {
            commands: interpret_linker.get_commands(name)?.iter(),
            context: Some(context),
            instrs: instrs.to_vec(),
            index: 0
        })
    }

    fn more_internal(&mut self) -> Result<bool,String> {
        let context = self.context.as_mut().unwrap();
        let idx = self.index;
        self.index += 1;
        if let Some(command) = self.commands.next() {
            let (instr,regs) = &self.instrs[idx];
            print!("{}",context.registers_mut().dump_many(&regs)?);
            print!("{}",instr);
            command.execute(context)?;
            context.registers_mut().commit();
            print!("{}",context.registers_mut().dump_many(&regs)?);
            if context.test_pause() {
                print!("PAUSE\n");
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn error_message(&self, msg: String) -> String {
        let context = self.context.as_ref().unwrap();
        let line = context.get_line_number();
        if line.1 != 0 {
            format!("{} at {}:{}",msg,line.0,line.1)
        } else {
            msg
        }
    }
}

impl<'a> InterpretInstance<'a> for DebugInterpretInstance<'a> {
    fn more(&mut self) -> Result<bool,String> {
        self.more_internal().map_err(|msg| self.error_message(msg))
    }

    fn finish(&mut self) -> InterpContext { self.context.take().unwrap() }
}
