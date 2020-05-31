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
use std::path::{ Path, PathBuf };
use std::time::{ SystemTime, Duration };
use std::rc::Rc;
use crate::cli::Config;
use crate::commands::{ make_core, make_library, make_buildtime };
use crate::generate::{ GenContext, Instruction };
use crate::model::Register;
use crate::interp::context::InterpContext;
use crate::interp::LibrarySuiteBuilder;
use crate::interp::{ InterpValue, StreamContents };
use super::compilelink::CompilerLink;
use super::interplink::InterpreterLink;
use crate::test::cbor::hexdump;

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

pub fn mini_interp_run(instrs: &Vec<Instruction>, cl: &CompilerLink, ic: &mut InterpContext, config: &Config) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    let program = cl.serialize(instrs,config)?;
    let mut buffer = Vec::new();
    serde_cbor::to_writer(&mut buffer,&program).expect("cbor b");
    print!("{}\n",hexdump(&buffer));

    let mut suite = LibrarySuiteBuilder::new();
    suite.add(make_core()?)?;
    suite.add(make_library()?)?;
    suite.add(make_buildtime()?)?;
    let interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x))?;

    if let Some(instrs) = interpret_linker.get_instructions() {
        /* debug info included */
        let mut instrs = instrs.iter();
        for command in interpret_linker.get_commands() {
            let (instr,regs) = instrs.next().unwrap();
            print!("{}",ic.registers().dump_many(&regs)?);
            print!("{}",instr);
            command.execute(ic)?;
            ic.registers().commit();
            print!("{}",ic.registers().dump_many(&regs)?);
        }
    } else {
        let start_time = SystemTime::now();
        for command in interpret_linker.get_commands() {
            command.execute(ic)?;
            ic.registers().commit();
        }
        print!("execution time {}ms\n",start_time.elapsed().unwrap_or(Duration::new(0,0)).as_secs_f32()*1000.);
    }
    Ok((export_indexes(ic)?,stream_strings(&ic.stream_take())))
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
    cfg.add_file_search_path("../src/commands/std/*.dp");
    cfg.add_file_search_path("../src/commands/buildtime/*.dp");
    cfg
}

pub fn mini_interp(instrs: &Vec<Instruction>, cl: &CompilerLink, config: &Config) -> Result<(HashMap<Register,Vec<usize>>,Vec<String>),String> {
    let mut ic = InterpContext::new();
    mini_interp_run(instrs,&cl,&mut ic,config).map_err(|x| {
        let line = ic.get_line_number();
        if line.1 != 0 {
            format!("{} at {}:{}",x,line.0,line.1)
        } else {
            x
        }
    })
}