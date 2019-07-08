use super::instruction::Instruction;
use super::register::{ Register, RegisterAllocator };
use super::definitionstore::DefStore;
use crate::parser::{ Statement, Expression };

struct Generator {
    regalloc: RegisterAllocator,
    defstore: DefStore,
    instrs: Vec<Instruction>
}

impl Generator {
    pub fn new(defstore: DefStore) -> Generator {
        Generator {
            regalloc: RegisterAllocator::new(),
            defstore,
            instrs: Vec::new()
        }
    }

    fn build_vec(&mut self, values: &Vec<Expression>) -> Register {
        let out = self.regalloc.allocate();
        self.instrs.push(Instruction::List(out.clone()));
        for val in values {
            let push = Instruction::Push(out.clone(),self.build_expr(val));
            self.instrs.push(push);
        }
        out
    }

    fn build_expr(&mut self, expr: &Expression) -> Register {
        match expr {
            Expression::Identifier(id) => Register::Named(id.to_string()),
            Expression::Number(n) => {
                let r = self.regalloc.allocate();
                self.instrs.push(Instruction::NumberConst(r.clone(),*n));
                r
            },
            Expression::Vector(v) => self.build_vec(v),
            _ => self.regalloc.allocate()
        }
    }

    fn build(&mut self, stmt: &Statement) {
        let regs : Vec<Register> = stmt.1.iter().map(|e| self.build_expr(e)).collect();
        self.instrs.push(Instruction::Proc(stmt.0.to_string(),regs))
    }

    pub fn go(mut self, stmts: Vec<Statement>) -> Vec<Instruction> {
        for stmt in &stmts {
            self.build(stmt);
        }
        self.instrs
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::Parser;
    use crate::testsuite::load_testdata;

    #[test]
    fn generate_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/generate-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let gen = Generator::new(defstore);
        let cmds : Vec<String> = gen.go(stmts).iter().map(|e| format!("{:?}",e)).collect();
        print!("{}",cmds.join(""));
    }
}