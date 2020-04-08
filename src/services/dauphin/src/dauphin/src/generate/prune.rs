use std::collections::HashSet;
use super::gencontext::GenContext;

pub fn prune(context: &mut GenContext) {
    let mut justified_calls = Vec::new();
    let mut justified_regs = HashSet::new();
    let mut rev_instrs = context.get_instructions();
    rev_instrs.reverse();
    for instr in rev_instrs {
        let mut call_justified = false;
        if instr.itype.self_justifying_call() {
            call_justified = true;
        }
        for idx in instr.itype.changing_registers() {
            if justified_regs.contains(&instr.regs[idx]) {
                call_justified = true;
                break;
            }
        }
        justified_calls.push(call_justified);
        if call_justified {
            for reg in instr.regs {
                justified_regs.insert(reg);
            }
        }
    }
    justified_calls.reverse();
    for (i,instr) in context.get_instructions().iter().enumerate() {
        if justified_calls[i] {
            context.add(instr.clone());
        }
    }
    context.phase_finished();
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::call;
    use super::super::simplify::simplify;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::generate::generate_code;
    use crate::interp::mini_interp;
    use super::super::linearize;
    use super::super::remove_aliases;

    // XXX test pruning, eg fewer lines
    #[test]
    fn prune_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        linearize(&mut context).expect("linearize");
        remove_aliases(&mut context);
        print!("{:?}\n",context);
        print!("PRUNE\n");
        prune(&mut context);
        print!("{:?}\n",context);
        let (_values,strings) = mini_interp(&mut context).expect("x");
        for s in &strings {
            print!("{}\n",s);
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }
}