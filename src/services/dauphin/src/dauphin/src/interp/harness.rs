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

use std::collections::{ HashMap };
use std::path::PathBuf;
use std::time::{ SystemTime, Duration };
use std::rc::Rc;
use crate::cli::Config;
use crate::commands::std_stream;
use crate::generate::Instruction;
use crate::model::Register;
use crate::interp::context::InterpContext;
use crate::interp::{ LibrarySuiteBuilder, make_librarysuite_builder, interpreter, InterpretInstance };
use crate::interp::{ InterpValue, StreamContents, StreamFactory };
use super::compilelink::CompilerLink;
use super::interplink::InterpreterLink;
use crate::test::cbor::hexdump;
use serde_cbor::Value as CborValue;

pub fn stream_strings(stream: &[StreamContents]) -> Vec<String> {
    let mut out = vec![];
    for s in stream {
        match s {
            StreamContents::String(s) => out.push(s.to_string()),
            _ => {}
        }
    }
    out
}

fn export_indexes(ic: &mut InterpContext) -> Result<HashMap<Register,Vec<usize>>,String> {
    let mut out = HashMap::new();
    for (r,iv) in ic.registers().export()?.iter() {
        let iv = Rc::new(iv.copy());
        let v = InterpValue::to_rc_indexes(&iv).map(|x| x.0.to_vec()).unwrap_or(vec![]);
        out.insert(*r,v);
    }
    Ok(out)
}

fn serialize(program: &CborValue) -> Result<Vec<u8>,String> {
    let mut buffer = Vec::new();
    serde_cbor::to_writer(&mut buffer,&program).map_err(|x| format!("{} while serialising",x))?;
    print!("{}\n",hexdump(&buffer));
    Ok(buffer)
}

#[cfg(test)]
pub fn mini_interp_run(interpret_linker: &InterpreterLink, config: &Config, name: &str) -> Result<InterpContext,String> {
    let mut interp = interpreter(interpret_linker,config,name)?;
    let start_time = SystemTime::now();
    let out = interpret(interpret_linker,config,name)?;
    print!("execution time {}ms\n",start_time.elapsed().unwrap_or(Duration::new(0,0)).as_secs_f32()*1000.);
    Ok(out)
}

#[cfg(test)]
pub fn interpret(interpret_linker: &InterpreterLink, config: &Config, name: &str) -> Result<InterpContext,String> {
    let mut interp = interpreter(interpret_linker,config,name)?;
    while interp.more()? {}
    Ok(interp.finish())
}

#[cfg(test)]
pub fn comp_interpret(compiler_linker: &CompilerLink, config: &Config, name: &str) -> Result<InterpContext,String> {
    let suite = make_librarysuite_builder(config)?;
    let program = compiler_linker.serialize(config)?;
    let mut interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x))?;
    interpret_linker.add_payload("std","stream",StreamFactory::new()); 
    interpret(&interpret_linker,config,name)
}

pub fn find_testdata() -> PathBuf {
    let mut dir = std::env::current_exe().expect("cannot get current exec path");
    while dir.pop() {
        let mut testdata = PathBuf::from(&dir);
        testdata.push("testdata");
        if testdata.exists() {
            return testdata;
        }
    }
    panic!("cannot find testdata directory");
}

pub fn xxx_test_config() -> Config {
    let mut cfg = Config::new();
    cfg.set_root_dir(&find_testdata().to_string_lossy());
    cfg.set_generate_debug(true);
    cfg.set_verbose(3);
    cfg.set_opt_level(2);
    cfg.set_debug_run(true);
    cfg.add_lib("buildtime");
    cfg.add_file_search_path("*.dp");
    cfg.add_file_search_path("parser/*.dp");
    cfg.add_file_search_path("parser/import-subdir/*.dp");
    cfg
}

#[cfg(test)]
pub fn mini_interp(instrs: &Vec<Instruction>, cl: &mut CompilerLink, config: &Config, name: &str) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    cl.add(name,instrs,config)?;
    let program = cl.serialize(config)?;
    let buffer = serialize(&program)?;
    let suite = make_librarysuite_builder(config)?;
    let program = serde_cbor::from_slice(&buffer).map_err(|x| format!("{} while deserialising",x))?;
    let mut interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x))?;
    interpret_linker.add_payload("std","stream",StreamFactory::new());
    let mut ic = mini_interp_run(&interpret_linker,config,name)?;
    let stream = std_stream(&mut ic)?;
    let strings = stream_strings(&stream.take());
    Ok((export_indexes(&mut ic)?,strings))
}