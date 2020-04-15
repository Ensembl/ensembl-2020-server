mod generate;
mod interp;
mod lexer;
mod model;
mod parser;
mod typeinf;

mod test {
    pub mod cbor;
    pub mod files;
}

#[macro_use]
extern crate lazy_static;
extern crate owning_ref;
extern crate serde_cbor;

/* This to remove RLS unused warns */

use crate::lexer::{ FileResolver, Lexer };
use crate::parser::Parser;
use crate::generate::generate_and_optimise;
use crate::test::files::load_testdata;
use crate::interp::mini_interp;

fn main() {
    let resolver = FileResolver::new();
    let mut lexer = Lexer::new(resolver);
    lexer.import("test:parser/parser-smoke.dp").expect("cannot load file");
    let p = Parser::new(lexer);
    let (stmts,defstore) = p.parse().map_err(|e| e[0].message().to_string()).expect("error");
    let mut out : Vec<String> = stmts.iter().map(|x| format!("{:?}",x)).collect();
    out.push("".to_string()); /* For trailing \n */
    let outdata = load_testdata(&["parser","parser-smoke.out"]).ok().unwrap();
    assert_eq!(outdata,out.join("\n"));
    let mut context = generate_and_optimise(&defstore,stmts).expect("codegen");
    mini_interp(&mut context).expect("A");
}
