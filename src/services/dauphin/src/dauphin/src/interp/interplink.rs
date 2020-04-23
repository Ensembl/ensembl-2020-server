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

use std::collections::BTreeMap;
use crate::generate::{ Instruction, InstructionType };
use crate::interp::commandsets::{ Command, CommandSchema, CommandInterpretSuite, CommandTrigger, CommandSuiteBuilder };
use crate::model::Register;
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_int, cbor_map, cbor_array, cbor_entry, cbor_string };

pub(super) const VERSION : u32 = 0;

struct ProgramCursor<'a> {
    value: &'a Vec<CborValue>,
    index: usize
}

impl<'a> ProgramCursor<'a> {
    fn more(&self) -> bool {
        self.index < self.value.len()
    }

    fn next(&mut self) -> Result<&'a CborValue,String> {
        if self.more() {
            self.index += 1;
            Ok(&self.value[self.index-1])
        } else {
            Err(format!("premature termination of program"))
        }
    }

    fn next_n(&mut self, n: usize) -> Result<Vec<&'a CborValue>,String> {
        (0..n).map(|i| self.next()).collect()
    }
}

pub struct InterpreterLink {
    commands: Vec<Box<dyn Command>>,
    instructions: Option<Vec<(String,Vec<Register>)>>
}

impl InterpreterLink {
    fn make_commands(ips: CommandInterpretSuite, program: &CborValue) -> Result<Vec<Box<dyn Command>>,String> {
        let mut cursor = ProgramCursor {
            value: cbor_array(program,0,true)?,
            index: 0
        };
        let mut out = vec![];
        while cursor.more() {
            let opcode = cbor_int(cursor.next()?,None)? as u32;
            let commandtype = ips.get_by_opcode(opcode)?;
            let num_args = commandtype.get_schema().values;
            let args = cursor.next_n(num_args)?;
            out.push(commandtype.deserialize(&args).map_err(|x| format!("{} while deserializing {}",x,commandtype.get_schema().trigger))?);
        }
        Ok(out)
    }

    fn make_instruction(cbor: &CborValue) -> Result<(String,Vec<Register>),String> {
        let data = cbor_array(cbor,2,false)?;
        let regs = cbor_array(&data[1],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        Ok((cbor_string(&data[0])?,regs))
    }

    fn make_instructions(cbor: &CborValue) -> Result<Vec<(String,Vec<Register>)>,String> {
        cbor_array(cbor,0,true)?.iter().map(|x| InterpreterLink::make_instruction(x)).collect()
    }

    pub fn new(cs: CommandSuiteBuilder, cbor: &CborValue) -> Result<InterpreterLink,String> {
        let data = cbor_map(cbor,&vec!["version","suite","program"])?;
        let got_ver = cbor_int(data[0],None)? as u32;
        if got_ver != VERSION {
            return Err(format!("Incompatible code. got v{} understand v{}",got_ver,VERSION));
        }
        let ips = cs.make_interpret_suite(data[1]).map_err(|x| format!("{} while building linker",x))?;
        Ok(InterpreterLink {
            commands: InterpreterLink::make_commands(ips,data[2]).map_err(|x| format!("{} while making commands",x))?,
            instructions: cbor_entry(cbor,"instructions")?.map(|x| InterpreterLink::make_instructions(x)).transpose()?
        })
    }

    pub fn get_commands(&self) -> &Vec<Box<dyn Command>> { &self.commands }
    pub fn get_instructions(&self) -> Option<&Vec<(String,Vec<Register>)>> { self.instructions.as_ref() }
}