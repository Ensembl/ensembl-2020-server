use super::intstruction2::Instruction2;
use crate::parser::{ Expression, Statement };
use crate::codegen::{ Register2, RegisterAllocator };
use crate::codegen::DefStore;
use crate::typeinf::{ ArgumentExpressionConstraint, SignatureMemberConstraint };

pub struct CodeGen {
    instrs: Vec<Instruction2>,
    regalloc: RegisterAllocator
}

impl CodeGen {
    pub fn new() -> CodeGen {
        CodeGen {
            instrs: Vec::new(),
            regalloc: RegisterAllocator::new()
        }
    }

    fn add_instr(&mut self, instr: Instruction2, defstore: &DefStore) -> Result<(),String> {
        //self.types.apply_command(&instr,defstore)?;
        self.instrs.push(instr);
        Ok(())
    }

    fn build_vec(&mut self, defstore: &DefStore, values: &Vec<Expression>, reg: &Register2) -> Result<(),String> {
        self.add_instr(Instruction2::List(reg.clone()),defstore)?;
        for val in values {
            let r = self.regalloc.allocate2();
            self.build_rvalue(defstore,val,&r)?;
            let push = Instruction2::Push(reg.clone(),r.clone());
            self.add_instr(push,defstore)?;
        }
        Ok(())
    }

    fn build_rvalue(&mut self, defstore: &DefStore, expr: &Expression, reg: &Register2) -> Result<(),String> {
        print!("ravlue for {:?}\n",expr);
        match expr {
            Expression::Number(n) => {
                self.add_instr(Instruction2::NumberConst(reg.clone(),*n),defstore)?;
            },
            Expression::LiteralString(s) => {
                self.add_instr(Instruction2::StringConst(reg.clone(),s.to_string()),defstore)?;
            },
            Expression::LiteralBool(b) => {
                self.add_instr(Instruction2::BooleanConst(reg.clone(),*b),defstore)?;
            },
            Expression::LiteralBytes(b) => {
                self.add_instr(Instruction2::BytesConst(reg.clone(),b.to_vec()),defstore)?;
            },
            Expression::Vector(v) => self.build_vec(defstore,v,reg)?,
            _ => {}
        }
        Ok(())
    }

    fn build_stmt(&mut self, defstore: &DefStore, stmt: &Statement) -> Result<(),String> {
        let mut regs = Vec::new();
        let procdecl = defstore.get_proc(&stmt.0);
        if procdecl.is_none() {
            return Err(format!("No such procedure '{}'",stmt.0));
        }
        for member in procdecl.unwrap().get_signature().each_member() {
            regs.push(self.regalloc.allocate2());
        }
        for (i,member) in procdecl.unwrap().get_signature().each_member().enumerate() {
            if let SignatureMemberConstraint::RValue(_) = member {
                self.build_rvalue(defstore,&stmt.1[i],&regs[i])?;
            }
        }
        self.add_instr(Instruction2::Proc(stmt.0.to_string(),regs.clone()),defstore)?;
        print!("{:?}\n",procdecl.unwrap().get_signature());
        Ok(())
    }

    pub fn go(mut self, defstore: &DefStore, stmts: Vec<Statement>) -> Result<Vec<Instruction2>,Vec<String>> {
        let mut errors = Vec::new();
        for stmt in &stmts {
            let r = self.build_stmt(defstore,stmt);
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
    fn codegen_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/generate-smoke2.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        //let regalloc = RegisterAllocator::new();
        let gen = CodeGen::new();
        let cmds : Vec<String> = gen.go(&defstore,stmts).expect("codegen").iter().map(|e| format!("{:?}",e)).collect();
        //let outdata = load_testdata(&["codegen","generate-smoke2.out"]).ok().unwrap();
        print!("{}",cmds.join("\n"));
        //assert_eq!(outdata,cmds.join(""));
    }
}
