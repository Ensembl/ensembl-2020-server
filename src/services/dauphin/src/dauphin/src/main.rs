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

/* This to remove RLS unused warns */

use crate::lexer::Lexer;
use crate::resolver::common_resolver;
use crate::parser::Parser;
use crate::generate::generate;
use crate::test::files::load_testdata;
use crate::interp::{ mini_interp, xxx_compiler_link, xxx_test_config };

fn main() {
    let resolver = common_resolver(&vec![]).expect("setting up path resolver");
    let mut lexer = Lexer::new(resolver);
    lexer.import("test:parser/parser-smoke.dp").expect("cannot load file");
    let p = Parser::new(lexer);
    let (stmts,defstore) = p.parse().map_err(|e| e[0].message().to_string()).expect("error");
    let mut out : Vec<String> = stmts.iter().map(|x| format!("{:?}",x)).collect();
    out.push("".to_string()); /* For trailing \n */
    let outdata = load_testdata(&["parser","parser-smoke.out"]).ok().unwrap();
    assert_eq!(outdata,out.join("\n"));
    let linker = xxx_compiler_link().expect("y");
    let config = xxx_test_config();
    let instrs = generate(&linker,&stmts,&defstore,&config).expect("codegen");
    mini_interp(&instrs,&linker).expect("A");
}
