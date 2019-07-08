mod codegen;
mod lexer;
mod parser;
mod testsuite;

/* This to remove RLS unused warns */

use crate::lexer::{ FileResolver, Lexer };
use crate::parser::Parser;
use crate::testsuite::load_testdata;

fn main() {
    let resolver = FileResolver::new();
    let mut lexer = Lexer::new(resolver);
    lexer.import("test:parser/parser-smoke.dp").expect("cannot load file");
    let p = Parser::new(lexer);
    let (stmts,_defstore) = p.parse().map_err(|e| e[0].message().to_string()).expect("error");
    let mut out : Vec<String> = stmts.iter().map(|x| format!("{:?}",x)).collect();
    out.push("".to_string()); /* For trailing \n */
    let outdata = load_testdata(&["parser","parser-smoke.out"]).ok().unwrap();
    assert_eq!(outdata,out.join("\n"));
}
