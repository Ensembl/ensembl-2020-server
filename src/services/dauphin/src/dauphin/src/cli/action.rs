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
use std::fs::write;
use std::process::exit;
use regex::Regex;
use crate::interp::{ make_librarysuite_builder, CompilerLink };
use crate::model::cbor_serialize;
use crate::lexer::Lexer;
use crate::parser::{ Parser, ParseError };
use crate::resolver::common_resolver;
use crate::generate::generate;
use super::Config;
use serde_cbor::Value as CborValue;
use serde_cbor::to_writer;

fn bomb<A,E,T>(action: T, x: Result<A,E>) -> A where T: Fn() -> String, E: Display {
    match x {
        Ok(v) => v,
        Err(e) => {
            eprint!("{} Error {}\n",action(),e.to_string());
            exit(2);
        }
    }
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

fn fix_filename(s: &str) -> String {
    let invalid = Regex::new("/").expect("bad regex in fix_filename");
    invalid.replace_all(s,"-").to_string()
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
        let builder = make_librarysuite_builder(&config).expect("y");
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
            make_librarysuite_builder(&config)
        );
        let mut linker = bomb(|| format!("cannot make linker"),
            CompilerLink::new(lib)
        );
        let resolver = bomb(|| format!("cannot create resolver"),
            common_resolver(&config,&linker)
        );
        for source in config.get_sources() {
            if config.get_verbose() > 0 {
                print!("compiling {}\n",source);
            }
            let mut lexer = Lexer::new(&resolver);
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
            let name = if let Some(name) = Regex::new(r".*/(.*?)\.dp").unwrap().captures_iter(source).next() {
                name.get(1).unwrap().as_str()
            } else {
                source
            };
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

pub(super) fn make_actions() -> HashMap<String,Box<dyn Action>> {
    let mut out : Vec<Box<dyn Action>> = vec![];
    out.push(Box::new(VersionAction()));
    out.push(Box::new(CompileAction()));
    out.push(Box::new(GenerateDynamicData()));
    out.drain(..).map(|a| (a.name(),a)).collect()
}

pub fn run(config: &Config) {
    let actions = make_actions();
    let action_name = config.get_action();
    if let Some(action) = actions.get(action_name) {
        action.execute(config);
    } else {
        eprint!("Invalid action '{}'\n",action_name);
    }
}