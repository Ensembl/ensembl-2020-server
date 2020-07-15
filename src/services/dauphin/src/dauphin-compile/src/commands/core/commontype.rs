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

#[macro_export]
macro_rules! type_instr2 {
    ($type:ident,$command:ident,$supertype:expr,$trial:ident) => {
        pub struct $type(Option<TimeTrial>);

        impl $type {
            fn new() -> $type { $type(None) }
        }

        impl CommandType for $type {
            fn get_schema(&self) -> CommandSchema {
                CommandSchema {
                    values: 2,
                    trigger: CommandTrigger::Instruction($supertype)
                }
            }
            fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
                Ok(Box::new($command(it.regs[0],it.regs[1],self.0.clone())))
            }

            fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
                let timings = TimeTrial::run(&$trial(),linker,config)?;
                Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
            }

            fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
                let t = cbor_map(value,&vec!["t"])?;
                self.0 = Some(TimeTrial::deserialize(&t[0])?);
                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! type_instr3 {
    ($type:ident,$command:ident,$supertype:expr,$trial:ident) => {
        pub struct $type(Option<TimeTrial>);

        impl $type {
            fn new() -> $type { $type(None) }
        }

        impl CommandType for $type {
            fn get_schema(&self) -> CommandSchema {
                CommandSchema {
                    values: 3,
                    trigger: CommandTrigger::Instruction($supertype)
                }
            }
            fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
                Ok(Box::new($command(it.regs[0],it.regs[1],it.regs[2],self.0.clone())))
            }

            fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
                let timings = TimeTrial::run(&$trial(),linker,config)?;
                Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
            }

            fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
                let t = cbor_map(value,&vec!["t"])?;
                self.0 = Some(TimeTrial::deserialize(&t[0])?);
                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! type_instr4 {
    ($type:ident,$command:ident,$supertype:expr,$trial:ident) => {
        pub struct $type(Option<TimeTrial>);

        impl $type {
            fn new() -> $type { $type(None) }
        }

        impl CommandType for $type {
            fn get_schema(&self) -> CommandSchema {
                CommandSchema {
                    values: 4,
                    trigger: CommandTrigger::Instruction($supertype)
                }
            }
            fn from_instruction(&self, it: &Instruction) -> Result<Box<dyn Command>,String> {
                Ok(Box::new($command(it.regs[0],it.regs[1],it.regs[2],it.regs[3],self.0.clone())))
            }

            fn generate_dynamic_data(&self, linker: &CompilerLink, config: &Config) -> Result<CborValue,String> {
                let timings = TimeTrial::run(&$trial(),linker,config)?;
                Ok(cbor_make_map(&vec!["t"],vec![timings.serialize()])?)
            }

            fn use_dynamic_data(&mut self, value: &CborValue) -> Result<(),String> {
                let t = cbor_map(value,&vec!["t"])?;
                self.0 = Some(TimeTrial::deserialize(&t[0])?);
                Ok(())
            }
        }
    };
}
