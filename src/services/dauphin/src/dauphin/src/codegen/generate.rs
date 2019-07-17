use std::collections::HashMap;

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

    fn build_vec(&mut self, values: &Vec<Expression>, dollar: &Option<Register>) -> Result<Register,String> {
        let out = self.regalloc.allocate();
        self.instrs.push(Instruction::List(out.clone()));
        for val in values {
            let push = Instruction::Push(out.clone(),self.build_expr(val,dollar)?);
            self.instrs.push(push);
        }
        Ok(out)
    }

    fn map_expressions(&mut self, x: &Vec<Expression>, dollar: &Option<Register>) -> Result<Vec<Register>,String> {
        x.iter().map(|e| self.build_expr(e,dollar)).collect()
    }

    fn struct_rearrange(&mut self, s: &str, x: Vec<Register>, got_names: &Vec<String>) -> Result<Vec<Register>,String> {
        if let Some(decl) = self.defstore.get_struct(s) {
            let gotpos : HashMap<String,usize> = got_names.iter().enumerate().map(|(i,e)| (e.to_string(),i)).collect();
            let mut out = Vec::new();
            for want_name in decl.get_names().iter() {
                if let Some(got_pos) = gotpos.get(want_name) {
                    out.push(x[*got_pos].clone());
                } else {
                    return Err(format!("Missing member '{}'",want_name));
                }
            }
            Ok(out)
        } else {
            Err(format!("no such struct '{}'",s))
        }
    }

    fn build_expr(&mut self, expr: &Expression, dollar: &Option<Register>) -> Result<Register,String> {
        Ok(match expr {
            Expression::Identifier(id) => Register::Named(id.to_string()),
            Expression::Number(n) => {
                let r = self.regalloc.allocate();
                self.instrs.push(Instruction::NumberConst(r.clone(),*n));
                r
            },
            Expression::LiteralString(s) => {
                let r = self.regalloc.allocate();
                self.instrs.push(Instruction::StringConst(r.clone(),s.to_string()));
                r
            },
            Expression::LiteralBool(b) => {
                let r = self.regalloc.allocate();
                self.instrs.push(Instruction::BooleanConst(r.clone(),*b));
                r
            },
            Expression::LiteralBytes(b) => {
                let r = self.regalloc.allocate();
                self.instrs.push(Instruction::BytesConst(r.clone(),b.to_vec()));
                r
            },
            Expression::Vector(v) => self.build_vec(v,dollar)?,
            Expression::CtorStruct(s,x,n) => {
                let r = self.regalloc.allocate();
                let x = self.map_expressions(x,dollar)?;
                let x = self.struct_rearrange(s,x,n)?;
                self.instrs.push(Instruction::CtorStruct(s.clone(),r.clone(),x));
                r
            },
            Expression::CtorEnum(e,b,x) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                self.instrs.push(Instruction::CtorEnum(e.clone(),b.clone(),r.clone(),x));
                r
            },
            Expression::Operator(name,x) => {
                let r = self.regalloc.allocate();
                let x = self.map_expressions(x,dollar)?;
                self.instrs.push(Instruction::Operator(name.clone(),x));
                r
            },
            Expression::Dot(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                self.instrs.push(Instruction::Dot(f.clone(),r.clone(),x));
                r
            },
            Expression::Query(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                self.instrs.push(Instruction::Query(f.clone(),r.clone(),x));
                r
            },
            Expression::Pling(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                self.instrs.push(Instruction::Pling(f.clone(),r.clone(),x));
                r
            },
            Expression::Square(x) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                self.instrs.push(Instruction::Square(r.clone(),x));
                r
            },
            Expression::Star(x) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                self.instrs.push(Instruction::Star(r.clone(),x));
                r
            },
            Expression::Filter(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                let f = self.build_expr(f,&Some(x.clone()))?;
                self.instrs.push(Instruction::Filter(r.clone(),x,f));
                r
            },
            Expression::Bracket(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_expr(x,dollar)?;
                let xsq = self.regalloc.allocate();
                self.instrs.push(Instruction::Square(xsq.clone(),x));
                let f = self.build_expr(f,&Some(xsq.clone()))?;
                let rm = self.regalloc.allocate();
                self.instrs.push(Instruction::Filter(rm.clone(),xsq,f));
                self.instrs.push(Instruction::Star(r.clone(),rm));
                r
            },
            Expression::Dollar => {
                if let Some(dollar) = dollar {
                    dollar.clone()
                } else {
                    return Err("Unexpected $".to_string());
                }
            },
            Expression::At => {
                if let Some(dollar) = dollar {
                    let r = self.regalloc.allocate();
                    self.instrs.push(Instruction::At(r.clone(),dollar.clone()));
                    r
                } else {
                    return Err("Unexpected $".to_string());
                }
            },
            _ => self.regalloc.allocate()
        })
    }
    // TODO deduplicate at

    fn build(&mut self, stmt: &Statement) -> Result<(),String> {
        let regs : Vec<Register> = self.map_expressions(&stmt.1,&None)?;
        self.instrs.push(Instruction::Proc(stmt.0.to_string(),regs));
        Ok(())
    }

    pub fn go(mut self, stmts: Vec<Statement>) -> Result<Vec<Instruction>,Vec<String>> {
        let mut errors = Vec::new();
        for stmt in &stmts {
            let r = self.build(stmt);
            if let Err(r) = r {
                errors.push(format!("{} at {} {}",r,stmt.2,stmt.3));
            }
        }
        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(self.instrs)
        }
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