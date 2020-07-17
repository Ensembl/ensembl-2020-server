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
use std::fmt::Display;
use std::fs::{ write, read };
use std::process::exit;
use regex::Regex;
use crate::suitebuilder::{ make_compiler_suite, make_interpret_suite };
use dauphin_interp::interp::InterpreterLink;
use dauphin_lib_std::{ StreamFactory };
use dauphin_interp::interp::{ InterpretInstance, DebugInterpretInstance, StandardInterpretInstance };
use dauphin_compile::model::{ fix_filename };
use dauphin_interp::common::{ cbor_serialize };
use dauphin_compile::lexer::{ Lexer };
use dauphin_compile::parser::{ Parser, ParseError };
use dauphin_compile::resolver::common_resolver;
use dauphin_compile::generate::generate;
use dauphin_compile::cli::Config;
use dauphin_compile::model::CompilerLink;
use serde_cbor::Value as CborValue;
use serde_cbor::to_writer;

pub fn interpreter<'a>(interpret_linker: &'a InterpreterLink, config: &Config, name: &str) -> Result<Box<dyn InterpretInstance<'a> + 'a>,String> {
    if let Some(instrs) = interpret_linker.get_instructions(name)? {
        if config.get_debug_run() {
            return Ok(Box::new(DebugInterpretInstance::new(interpret_linker,&instrs,name)?));
        }
    }
    Ok(Box::new(StandardInterpretInstance::new(interpret_linker,name)?))
}

fn bomb<A,E,T>(action: T, x: Result<A,E>) -> A where T: Fn() -> String, E: Display {
    match x {
        Ok(v) => v,
        Err(e) => {
            eprint!("{} Error {}\n",action(),e.to_string());
            exit(2);
        }
    }
}

fn read_binary_file(filename: &str) -> Vec<u8> {
    bomb(
        || format!("Reading {}",filename),
        read(filename)
    )
}

fn write_binary_file(filename: &str, contents: &[u8]) {
    bomb(
        || format!("Writing {}",filename),
        write(filename,contents)
    )
}

fn write_cbor_file(filename: &str, contents: &CborValue) {
    let mut buffer = Vec::new();
    bomb(
        || format!("while serialising CBOR for {}",filename),
        to_writer(&mut buffer,&contents).map_err(|x| format!("{} while serialising",x))
    );
    write_binary_file(filename,&buffer);
}

pub trait Action {
    fn name(&self) -> String;
    fn execute(&self, config: &Config);
}

struct VersionAction();

impl Action for VersionAction {
    fn name(&self) -> String { "version".to_string() }
    fn execute(&self, _: &Config) {
        print!("0.0\n");
    }
}

struct GenerateDynamicData();

impl Action for GenerateDynamicData {
    fn name(&self) -> String { "generate-dynamic-data".to_string() }
    fn execute(&self, config: &Config) {
        let builder = make_compiler_suite(&config).expect("y");
        let linker = CompilerLink::new(builder).expect("z");
        let data = linker.generate_dynamic_data(&config).expect("x");
        for (suite,data) in data.iter() {
            print!("writing data for {}\n",suite);
            write_cbor_file(&format!("{}.ddd",fix_filename(&suite.to_string())),data);
        }
    }
}

fn format_parse_errors(x: &[ParseError]) -> String {
    x.iter().map(|x| x.message()).collect::<Vec<_>>().join("\n")
}

struct CompileAction();

impl Action for CompileAction {
    fn name(&self) -> String { "compile".to_string() }
    fn execute(&self, config: &Config) {
        let lib = bomb(|| format!("cannot make library suite"),
            make_compiler_suite(&config)
        );
        let mut linker = bomb(|| format!("cannot make linker"),
            CompilerLink::new(lib)
        );
        let resolver = bomb(|| format!("cannot create resolver"),
            common_resolver(&config,&linker)
        );
        for source in config.get_sources() {
            let name = if let Some(name) = Regex::new(r".*/(.*?)\.dp").unwrap().captures_iter(source).next() {
                name.get(1).unwrap().as_str()
            } else {
                source
            };
            if config.get_verbose() > 0 {
                print!("compiling {}\n",source);
            }
            let mut lexer = Lexer::new(&resolver,name);
            bomb(|| format!("cannot load {}",source),
                lexer.import(&format!("file:{}",source))
            );
            let p = Parser::new(&mut lexer);
            let (stmts,defstore) = bomb(|| format!("cannot compile {}\n",source),
                p.parse().map_err(|x| format_parse_errors(&x))
            );
            let instrs = bomb(|| format!("cannot generate binary for {}",source),
                generate(&linker,&stmts,&defstore,&resolver,&config)
            );
            bomb(|| format!("cannot add instructions to binary for {}",source),
                linker.add(&name,&instrs,config)
            );
        }
        let program = bomb(|| format!("cannot serialize program to CBOR"),
            linker.serialize(config)
        );
        let buffer = bomb(|| format!("cannot serialize CBOR to byes"),
            cbor_serialize(&program)
        );
        write_binary_file(config.get_output(),&buffer);
    }
}

struct RunAction();

impl Action for RunAction {
    fn name(&self) -> String { "run".to_string() }
    fn execute(&self, config: &Config) {
        let suite = bomb(|| format!("could not construct library"),
            make_interpret_suite(config)
        );
        let buffer = read_binary_file(config.get_output());
        let program = bomb(|| format!("corrupted cbor in {}",config.get_output()),
                        serde_cbor::from_slice(&buffer).map_err(|x| format!("{} while deserialising",x)));
        let mut interpret_linker = bomb(|| format!("could not link binary"),
            InterpreterLink::new(suite,&program)
        );
        let mut sf = StreamFactory::new();
        sf.to_stdout(true);
        interpret_linker.add_payload("std","stream",sf);
        let mut interp = interpreter(&interpret_linker,&config,config.get_run()).expect("interpreter");
        while interp.more().expect("interpreting") {}
        interp.finish();    
    }
}

pub(super) fn make_actions() -> HashMap<String,Box<dyn Action>> {
    let mut out : Vec<Box<dyn Action>> = vec![];
    out.push(Box::new(VersionAction()));
    out.push(Box::new(CompileAction()));
    out.push(Box::new(GenerateDynamicData()));
    out.push(Box::new(RunAction()));
    out.drain(..).map(|a| (a.name(),a)).collect()
}

pub fn run(config: &Config) {
    bomb(|| format!("bad config"), config.verify());
    let actions = make_actions();
    let action_name = config.get_action();
    if let Some(action) = actions.get(action_name) {
        action.execute(config);
    } else {
        eprint!("Invalid action '{}'\n",action_name);
    }
}