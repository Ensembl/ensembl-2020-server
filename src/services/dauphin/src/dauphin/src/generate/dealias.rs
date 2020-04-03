use std::collections::HashMap;
use crate::model::Register;
use super::gencontext::GenContext;
use super::instruction::{ Instruction, InstructionType };

struct Aliases(HashMap<Register,Register>);

impl Aliases {
    fn lookup(&self, alias: &Register) -> Register {
        match self.0.get(&alias) {
            Some(further) => self.lookup(further),
            None => *alias
        }
    }

    fn alias(&mut self, alias: &Register, target: &Register) {
        self.0.insert(*alias,self.lookup(target));
    }
}

pub fn remove_aliases(context: &mut GenContext) {
    let mut aliases = Aliases(HashMap::new());
    for instr in context.get_instructions() {
        match instr.itype {
            InstructionType::Alias => {
                aliases.alias(&instr.regs[0],&instr.regs[1]);
            },
            _ => {
                context.add(Instruction::new(instr.itype,instr.regs.iter().map(|x| aliases.lookup(x)).collect()));
            }
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

    #[test]
    fn dealias_smoke() {
        // XXX check all aliases gone
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/linearize-refsquare.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let mut context = generate_code(&defstore,stmts).expect("codegen");
        call(&mut context).expect("j");
        simplify(&defstore,&mut context).expect("k");
        print!("{:?}\n",context);
        linearize(&mut context).expect("linearize");
        print!("BEFORE {:?}\n",context);
        remove_aliases(&mut context);
        print!("AFTER {:?}\n",context);
        let (_prints,values,strings) = mini_interp(&mut context);
        print!("{:?}\n",values);
        for s in &strings {
            print!("{}\n",s);
        }
        for instr in context.get_instructions() {
            if let InstructionType::Alias = instr.itype {
                assert!(false);
            }
        }
        assert_eq!(vec!["[[0],[2],[0],[4]]","[[0],[2],[9,9,9],[9,9,9]]","[0,0,0]","[[0],[2],[8,9,9],[9,9,9]]"],strings);
    }

}