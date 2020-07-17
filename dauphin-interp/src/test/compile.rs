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

use std::time::{ SystemTime, Duration };
use std::collections::HashMap;
use std::rc::Rc;
use dauphin_compile::cli::Config;
use dauphin_compile::command::{ CommandCompileSuite, CompilerLink, Instruction };
use crate::command::{ CommandInterpretSuite, InterpreterLink };
use crate::{ make_core_interp };
use dauphin_lib_std::{ make_std_interp };
use crate::runtime::{ InterpContext, InterpValue, Register };
use dauphin_lib_std::stream::{ StreamFactory, Stream };
use crate::util::cbor::{ cbor_serialize };
use dauphin_compile::core::{ make_core };
use dauphin_compile::generate::generate;
use dauphin_compile::resolver::common_resolver;
use dauphin_compile::lexer::Lexer;
use dauphin_compile::parser::Parser;
use crate::test::cbor::hexdump;
use crate::runtime::{ StandardInterpretInstance, DebugInterpretInstance, InterpretInstance };

pub fn interpreter<'a>(interpret_linker: &'a InterpreterLink, config: &Config, name: &str) -> Result<Box<dyn InterpretInstance<'a> + 'a>,String> {
    if let Some(instrs) = interpret_linker.get_instructions(name)? {
        if config.get_debug_run() {
            return Ok(Box::new(DebugInterpretInstance::new(interpret_linker,&instrs,name)?));
        }
    }
    Ok(Box::new(StandardInterpretInstance::new(interpret_linker,name)?))
}

fn export_indexes(ic: &mut InterpContext) -> Result<HashMap<Register,Vec<usize>>,String> {
    let mut out = HashMap::new();
    for (r,iv) in ic.registers_mut().export()?.iter() {
        let iv = Rc::new(iv.copy());
        let v = InterpValue::to_rc_indexes(&iv).map(|x| x.0.to_vec()).unwrap_or(vec![]);
        out.insert(*r,v);
    }
    Ok(out)
}

pub fn std_stream(context: &mut InterpContext) -> Result<&mut Stream,String> {
    let p = context.payload("std","stream")?;
    Ok(p.downcast_mut().ok_or_else(|| "No stream context".to_string())?)
}

pub fn interpret(interpret_linker: &InterpreterLink, config: &Config, name: &str) -> Result<InterpContext,String> {
    let mut interp = interpreter(interpret_linker,config,name)?;
    while interp.more()? {}
    Ok(interp.finish())
}

pub fn mini_interp_run(interpret_linker: &InterpreterLink, config: &Config, name: &str) -> Result<InterpContext,String> {
    let interp = interpreter(interpret_linker,config,name)?;
    let start_time = SystemTime::now();
    let out = interpret(interpret_linker,config,name)?;
    print!("command time {}ms\n",start_time.elapsed().unwrap_or(Duration::new(0,0)).as_secs_f32()*1000.);
    Ok(out)
}

pub fn make_interpret_suite() -> Result<CommandInterpretSuite,String> {
    let mut suite = CommandInterpretSuite::new();
    suite.register(make_core_interp()?)?;
    Ok(suite)
}

pub fn make_compiler_suite(config: &Config) -> Result<CommandCompileSuite,String> {
    let mut suite = CommandCompileSuite::new();
    suite.register(make_core()?)?;
    Ok(suite)
}
