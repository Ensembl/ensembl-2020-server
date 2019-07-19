mod codegen;
mod lexer;
mod parser;
mod testsuite;

#[macro_use]
extern crate lazy_static;

/* This to remove RLS unused warns */

use crate::lexer::{ FileResolver, Lexer };
use crate::parser::Parser;
use crate::codegen::{ Generator, TypePass };
use crate::testsuite::load_testdata;

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
    //
    let gen = Generator::new();
    let instrs = gen.go(&defstore,stmts).expect("codegen");
    let mut tp = TypePass::new();
    for instr in &instrs {
        print!("=== {:?}",instr);
        tp.apply_command(instr,&defstore).expect("ok");
        //print!("finish {:?}\n",tp.typeinf);
    }
}
