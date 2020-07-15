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

use std::collections::HashMap;
use std::rc::Rc;
use crate::interp::{ CommandInterpretSuite };
use serde_cbor::Value as CborValue;
use dauphin_interp_common::common::{ cbor_int, cbor_map, cbor_array, cbor_entry, cbor_string, cbor_map_iter, Register, InterpCommand };
use dauphin_interp_common::interp::{ InterpContext, PayloadFactory };

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
        (0..n).map(|_| self.next()).collect()
    }
}

pub struct InterpreterLinkProgram {
    commands: Vec<Box<dyn InterpCommand>>,
    instructions: Option<Vec<(String,Vec<Register>)>>
}

pub struct InterpreterLink {
    programs: HashMap<String,InterpreterLinkProgram>,
    payloads: HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>
}

impl InterpreterLink {
    fn make_commands(ips: &CommandInterpretSuite, program: &CborValue) -> Result<Vec<Box<dyn InterpCommand>>,String> {
        let mut cursor = ProgramCursor {
            value: cbor_array(program,0,true)?,
            index: 0
        };
        let mut out = vec![];
        while cursor.more() {
            let opcode = cbor_int(cursor.next()?,None)? as u32;
            let ds = ips.get_deserializer(opcode)?;
            let (_,num_args) = ds.get_opcode_len()?.ok_or_else(|| format!("attempt to deserialize an unserializable"))?;
            let args = cursor.next_n(num_args)?;
            out.push(ds.deserialize(opcode,&args).map_err(|x| format!("{} while deserializing",x))?);
        }
        Ok(out)
    }

    pub fn add_payload<P>(&mut self, set: &str, name: &str, pf: P) where P: PayloadFactory + 'static {
        self.payloads.insert((set.to_string(),name.to_string()),Rc::new(Box::new(pf)));
    }

    fn make_instruction(cbor: &CborValue) -> Result<(String,Vec<Register>),String> {
        let data = cbor_array(cbor,2,false)?;
        let regs = cbor_array(&data[1],0,true)?.iter().map(|x| Register::deserialize(x)).collect::<Result<_,_>>()?;
        Ok((cbor_string(&data[0])?,regs))
    }

    fn make_instructions(cbor: &CborValue) -> Result<Vec<(String,Vec<Register>)>,String> {
        cbor_array(cbor,0,true)?.iter().map(|x| InterpreterLink::make_instruction(x)).collect()
    }

    fn get_program<'a>(&'a self, name: &str) -> Result<&'a InterpreterLinkProgram,String> {
        Ok(self.programs.get(name).ok_or_else(|| format!("No such program {}",name))?)
    }

    pub fn new(mut ips: CommandInterpretSuite, cbor: &CborValue) -> Result<InterpreterLink,String> {
        let mut out = InterpreterLink {
            programs: HashMap::new(),
            payloads: ips.copy_payloads()
        };
        let data = cbor_map(cbor,&vec!["version","suite","programs"])?;
        ips.adjust(data[1])?;
        let got_ver = cbor_int(data[0],None)? as u32;
        if got_ver != VERSION {
            return Err(format!("Incompatible code. got v{} understand v{}",got_ver,VERSION));
        }
        for (name,program) in cbor_map_iter(data[2])? {
            let name = cbor_string(name)?;
            let cmds = cbor_entry(program,"cmds")?.ok_or_else(|| "bad cbor: no cmds section".to_string())?;
            let symbols = cbor_entry(program,"symbols")?;
            out.programs.insert(name.to_string(),InterpreterLinkProgram {
                commands: InterpreterLink::make_commands(&ips,cmds).map_err(|x| format!("{} while making commands",x))?,
                instructions: symbols.map(|x| InterpreterLink::make_instructions(x)).transpose()?
            });
        }
        Ok(out)
    }

    pub fn get_commands(&self, name: &str) -> Result<&Vec<Box<dyn InterpCommand>>,String> {
        Ok(&self.get_program(name)?.commands)
    }

    pub fn get_instructions(&self, name: &str) -> Result<Option<&Vec<(String,Vec<Register>)>>,String> { 
        Ok(self.get_program(name)?.instructions.as_ref())
    }

    pub(super) fn new_context(&self) -> InterpContext {
        InterpContext::new(&self.payloads)
    }
}

#[cfg(test)]
mod test {
    use dauphin_compile_common::cli::Config;
    use dauphin_compile_common::model::CompilerLink;
    use dauphin_compile::lexer::Lexer;
    use dauphin_compile::resolver::common_resolver;
    use dauphin_compile::parser::{ Parser };
    use dauphin_compile::resolver::Resolver;
    use dauphin_compile::generate::generate;
    use crate::test::{ mini_interp_run, xxx_test_config, make_compiler_suite, make_interpret_suite };
    use dauphin_interp_common::interp::{ InterpContext };
    use dauphin_lib_std_interp::stream::{ StreamFactory, Stream };
    use crate::interp::interplink::InterpreterLink;

    pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
        let p = context.payload("std","stream")?;
        Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
    }
    
    fn make_program(linker: &mut CompilerLink, resolver: &Resolver, config: &Config, name: &str, path: &str) -> Result<(),String> {
        let mut lexer = Lexer::new(&resolver,"");
        lexer.import(path).expect("cannot load file");
        let p = Parser::new(&mut lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("j");
        linker.add(name,&instrs,config)?;
        Ok(())
    }

    #[test]
    fn test_multi_program() {
        let mut config = xxx_test_config();
        config.set_generate_debug(false);
        config.set_verbose(2);
        let mut linker = CompilerLink::new(make_compiler_suite(&config).expect("y")).expect("y2");
        let resolver = common_resolver(&config,&linker).expect("a");
        make_program(&mut linker,&resolver,&config,"prog1","search:codegen/multiprog1").expect("cannot build prog1");
        make_program(&mut linker,&resolver,&config,"prog2","search:codegen/multiprog2").expect("cannot build prog2");
        make_program(&mut linker,&resolver,&config,"prog3","search:codegen/multiprog3").expect("cannot build prog3");
        let program = linker.serialize(&config).expect("serialize");
        let suite = make_interpret_suite().expect("c");
        let mut interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x)).expect("d");
        interpret_linker.add_payload("std","stream",StreamFactory::new());
        let mut ic_a = mini_interp_run(&interpret_linker,&config,"prog2").expect("A");
        let mut ic_b = mini_interp_run(&interpret_linker,&config,"prog1").expect("B");
        let s_a = std_stream(&mut ic_a).expect("d");
        let s_b = std_stream(&mut ic_b).expect("e");
        let a = &s_a.take();
        let b = &s_b.take();    
        assert_eq!(vec!["prog2"],a.iter().map(|x| x).collect::<Vec<_>>());
        assert_eq!(vec!["prog1"],b.iter().map(|x| x).collect::<Vec<_>>());
    }
}
