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
use crate::interp::{ LibrarySuiteBuilder, make_librarysuite_builder };
use crate::interp::{ InterpValue, StreamContents, StreamFactory };
use super::compilelink::CompilerLink;
use super::interplink::InterpreterLink;
use crate::test::cbor::hexdump;
use serde_cbor::Value as CborValue;

fn stream_strings(stream: &[StreamContents]) -> Vec<String> {
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

pub fn mini_interp_run(interpret_linker: &InterpreterLink, ic: &mut InterpContext, config: &Config, name: &str) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    if let Some(instrs) = interpret_linker.get_instructions(name)? {
        /* debug info included */
        let mut instrs = instrs.iter();
        for command in interpret_linker.get_commands(name)? {
            let (instr,regs) = instrs.next().unwrap();
            print!("{}",ic.registers().dump_many(&regs)?);
            print!("{}",instr);
            command.execute(ic)?;
            ic.registers().commit();
            print!("{}",ic.registers().dump_many(&regs)?);
        }
    } else {
        let commands = interpret_linker.get_commands(name)?;
        let start_time = SystemTime::now();
        for command in commands {
            command.execute(ic)?;
            ic.registers().commit();
        }
        print!("execution time {}ms\n",start_time.elapsed().unwrap_or(Duration::new(0,0)).as_secs_f32()*1000.);
    }
    let stream = std_stream(ic)?;
    let strings = stream_strings(&stream.take());
    Ok((export_indexes(ic)?,strings))
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
    cfg.add_lib("buildtime");
    cfg.add_file_search_path("*.dp");
    cfg.add_file_search_path("parser/*.dp");
    cfg.add_file_search_path("parser/import-subdir/*.dp");
    cfg
}

pub fn mini_interp(instrs: &Vec<Instruction>, cl: &mut CompilerLink, config: &Config, name: &str) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    cl.add(name,instrs,config)?;
    let program = cl.serialize(config)?;
    let buffer = serialize(&program)?;
    let suite = make_librarysuite_builder(config)?;
    let program = serde_cbor::from_slice(&buffer).map_err(|x| format!("{} while deserialising",x))?;
    let mut interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x))?;
    interpret_linker.add_payload("std","stream",StreamFactory::new());
    let mut ic = interpret_linker.new_context();
    mini_interp_run(&interpret_linker,&mut ic,config,name).map_err(|x| {
        let line = ic.get_line_number();
        if line.1 != 0 {
            format!("{} at {}:{}",x,line.0,line.1)
        } else {
            x
        }
    })
}