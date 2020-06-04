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

mod commands;
mod cli;
mod generate;
mod interp;
mod lexer;
mod model;
mod parser;
mod resolver;
mod typeinf;

mod test {
    pub mod cbor;
    pub mod files;
}

#[macro_use]
extern crate lazy_static;
extern crate owning_ref;
extern crate serde_cbor;
extern crate crc;
extern crate ini;

/* This to remove RLS unused warns */

use crate::lexer::Lexer;
use crate::resolver::common_resolver;
use crate::parser::Parser;
use crate::generate::generate;
use crate::test::files::load_testdata;
use crate::interp::{ CompilerLink, xxx_test_config, make_librarysuite_builder };
use crate::interp::{ LibrarySuiteBuilder, interpreter, InterpreterLink, StreamFactory };

fn main() {
    let config = xxx_test_config();
    let mut linker = CompilerLink::new(make_librarysuite_builder(&config).expect("y")).expect("y2");
    let resolver = common_resolver(&config,&linker).expect("setting up path resolver");
    let mut lexer = Lexer::new(&resolver);
    lexer.import("search:parser/parser-smoke.dp").expect("cannot load file");
    let p = Parser::new(&mut lexer);
    let (stmts,defstore) = p.parse().map_err(|e| e[0].message().to_string()).expect("error");
    let mut out : Vec<String> = stmts.iter().map(|x| format!("{:?}",x)).collect();
    out.push("".to_string()); /* For trailing \n */
    let outdata = load_testdata(&["parser","parser-smoke.out"]).ok().unwrap();
    assert_eq!(outdata,out.join("\n"));
    let instrs = generate(&linker,&stmts,&defstore,&resolver,&config).expect("codegen");
    let suite = make_librarysuite_builder(&config).expect("suite");
    linker.add("main",&instrs,&config).expect("adding program");
    let program = linker.serialize(&config).expect("serialize");
    let mut interpret_linker = InterpreterLink::new(suite,&program).map_err(|x| format!("{} while linking",x)).expect("linking");
    interpret_linker.add_payload("std","stream",StreamFactory::new()); 
    let mut interp = interpreter(&interpret_linker,&config,"main").expect("interpreter");
    while interp.more().expect("interpreting") {}
    interp.finish();
}
