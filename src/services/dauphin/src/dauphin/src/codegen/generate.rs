use std::collections::HashMap;

use super::instruction::Instruction;
use super::register::{ Register, RegisterAllocator };
use super::definitionstore::DefStore;
use crate::parser::{ Statement, Expression };
use crate::types::{ TypePass, Referrer, TypeSigExpr, BaseType };

pub struct Generator {
    types: TypePass,
    regalloc: RegisterAllocator,
    instrs: Vec<Instruction>
}

impl Generator {
    pub fn new(regalloc: &RegisterAllocator) -> Generator {
        Generator {
            types: TypePass::new(true),
            regalloc: regalloc.clone(),
            instrs: Vec::new()
        }
    }

    fn add_instr(&mut self, instr: Instruction, defstore: &DefStore) -> Result<(),String> {
        self.types.apply_command(&instr,defstore)?;
        self.instrs.push(instr);
        Ok(())
    }

    fn build_vec(&mut self, defstore: &DefStore, values: &Vec<Expression>, dollar: &Option<Register>) -> Result<Register,String> {
        let out = self.regalloc.allocate();
        self.add_instr(Instruction::List(out.clone()),defstore)?;
        for val in values {
            let push = Instruction::Push(out.clone(),self.build_rvalue(defstore,val,dollar)?);
            self.add_instr(push,defstore)?;
        }
        Ok(out)
    }

    fn map_expressions(&mut self, defstore: &DefStore, x: &Vec<Expression>, dollar: &Option<Register>) -> Result<Vec<Register>,String> {
        x.iter().map(|e| self.build_rvalue(defstore,e,dollar)).collect()
    }

    fn map_expressions_top(&mut self, defstore: &DefStore, x: &Vec<Expression>, lvalues: &Vec<bool>) -> Result<Vec<Register>,String> {
        let mut out = Vec::new();
        for (i,e) in x.iter().enumerate() {
            out.push(if lvalues[i] {
                //self.build_lvalue(defstore,e)?
                self.regalloc.allocate()
            } else {
                self.build_rvalue(defstore,e,&None)?
            });
        }
        Ok(out)
    }

    fn map_lvalues_top(&mut self, defstore: &DefStore, x: &Vec<Expression>, regs: &Vec<Register>, lvalues: &Vec<bool>) -> Result<(),String> {
        for (i,e) in x.iter().enumerate() {
            if lvalues[i] {
                self.build_lvalue2(defstore,e,&regs[i])?;
            }
        }
        Ok(())
    }

    fn struct_rearrange(&mut self, defstore: &DefStore, s: &str, x: Vec<Register>, got_names: &Vec<String>) -> Result<Vec<Register>,String> {
        if let Some(decl) = defstore.get_struct(s) {
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

    fn type_of(&mut self, defstore: &DefStore, expr: &Expression) -> Result<TypeSigExpr,String> {
        Ok(match expr {
            Expression::Identifier(id) => {
                self.types.get_typeinf().get_sig(&Referrer::Register(Register::Named(id.to_string()))).expr().clone()
            },
            Expression::Dot(x,f) => {
                if let TypeSigExpr::Base(BaseType::StructType(name)) = self.type_of(defstore,x)? {
                    if let Some(struct_) = defstore.get_struct(&name) {
                        if let Some(type_) = struct_.get_member_type(f) {
                            type_.to_typesigexpr()
                        } else {
                            return Err(format!("no such field {:?}",f));
                        }
                    } else {
                        return Err(format!("{:?} is not a structure",expr));
                    }
                } else {
                    return Err(format!("{:?} is not a structure",expr));
                }
            },
            _ => return Err(format!("Cannot type {:?}",expr))
        })
    }

    fn build_lvalue2(&mut self, defstore: &DefStore, expr: &Expression, r: &Register) -> Result<(),String> {
        match expr {
            Expression::Identifier(id) => {
                let a = Register::Named(id.to_string());
                self.add_instr(Instruction::Set(a,r.clone()),defstore)?;
            },
            Expression::Dot(x,f) => {
                if let TypeSigExpr::Base(BaseType::StructType(struct_name)) = self.type_of(defstore,x)? {
                    let tmp = self.regalloc.allocate();
                    self.add_instr(Instruction::RefSValue2(f.clone(),struct_name.to_string(),tmp.clone(),r.clone()),defstore)?;
                    self.build_lvalue2(defstore,x,&tmp)?;
                }
            },
            _ => return Err("Invalid lvalue".to_string())
        };
        Ok(())
    }

    fn build_lvalue(&mut self, defstore: &DefStore, expr: &Expression) -> Result<Register,String> {
        Ok(match expr {
            Expression::Identifier(id) => {
                let r = self.regalloc.allocate();
                let a = Register::Named(id.to_string());
                self.add_instr(Instruction::Ref(r.clone(),a),defstore)?;
                r
            },
            Expression::Dot(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_lvalue(defstore,x)?;
                let stype = self.types.get_typeinf().get_sig(&Referrer::Register(x.clone())).clone();
                if let TypeSigExpr::Base(BaseType::StructType(name)) = stype.expr() {
                    self.add_instr(Instruction::RefSValue(f.clone(),name.to_string(),r.clone(),x),defstore)?;
                } else {
                    return Err("Can only take \"dot\" of structs".to_string());
                }
                r
            },
            Expression::Pling(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_lvalue(defstore,x)?;
                let etype = self.types.get_typeinf().get_sig(&Referrer::Register(x.clone())).clone();
                if let TypeSigExpr::Base(BaseType::EnumType(name)) = etype.expr() {
                    self.add_instr(Instruction::RefEValue(f.clone(),name.to_string(),r.clone(),x),defstore)?;
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
                r
            },
            Expression::Square(x) => {
                let r = self.regalloc.allocate();
                let x = self.build_lvalue(defstore,x)?;
                self.add_instr(Instruction::RefSquare(r.clone(),x),defstore)?;
                r
            },
            Expression::Filter(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_lvalue(defstore,x)?;
                let f = self.build_rvalue(defstore,f,&Some(x.clone()))?;
                self.add_instr(Instruction::RefFilter(r.clone(),x,f),defstore)?;
                r
            },
            Expression::Bracket(x,f) => {
                let x = self.build_lvalue(defstore,x)?;
                let xsq = self.regalloc.allocate();
                self.add_instr(Instruction::RefSquare(xsq.clone(),x),defstore)?;
                let f = self.build_rvalue(defstore,f,&Some(xsq.clone()))?;
                let r = self.regalloc.allocate();
                self.add_instr(Instruction::RefFilter(r.clone(),xsq,f),defstore)?;
                r
            },
            _ => return Err("Invalid lvalue".to_string())
        })
    }

    fn build_rvalue(&mut self, defstore: &DefStore, expr: &Expression, dollar: &Option<Register>) -> Result<Register,String> {
        Ok(match expr {
            Expression::Identifier(id) => Register::Named(id.to_string()),
            Expression::Number(n) => {
                let r = self.regalloc.allocate();
                self.add_instr(Instruction::NumberConst(r.clone(),*n),defstore)?;
                r
            },
            Expression::LiteralString(s) => {
                let r = self.regalloc.allocate();
                self.add_instr(Instruction::StringConst(r.clone(),s.to_string()),defstore)?;
                r
            },
            Expression::LiteralBool(b) => {
                let r = self.regalloc.allocate();
                self.add_instr(Instruction::BooleanConst(r.clone(),*b),defstore)?;
                r
            },
            Expression::LiteralBytes(b) => {
                let r = self.regalloc.allocate();
                self.add_instr(Instruction::BytesConst(r.clone(),b.to_vec()),defstore)?;
                r
            },
            Expression::Vector(v) => self.build_vec(defstore,v,dollar)?,
            Expression::CtorStruct(s,x,n) => {
                let r = self.regalloc.allocate();
                let x = self.map_expressions(defstore,x,dollar)?;
                let x = self.struct_rearrange(defstore,s,x,n)?;
                self.add_instr(Instruction::CtorStruct(s.clone(),r.clone(),x),defstore)?;
                r
            },
            Expression::CtorEnum(e,b,x) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                self.add_instr(Instruction::CtorEnum(e.clone(),b.clone(),r.clone(),x),defstore)?;
                r
            },
            Expression::Operator(name,x) => {
                let r = self.regalloc.allocate();
                let x = self.map_expressions(defstore,x,dollar)?;
                self.add_instr(Instruction::Operator(name.clone(),r.clone(),x),defstore)?;
                r
            },
            Expression::Dot(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                let stype = self.types.get_typeinf().get_sig(&Referrer::Register(x.clone())).clone();
                if let TypeSigExpr::Base(BaseType::StructType(name)) = stype.expr() {
                    self.add_instr(Instruction::SValue(f.clone(),name.to_string(),r.clone(),x),defstore)?;
                } else {
                    return Err("Can only take \"dot\" of structs".to_string());
                }
                r
            },
            Expression::Query(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                let etype = self.types.get_typeinf().get_sig(&Referrer::Register(x.clone())).clone();
                if let TypeSigExpr::Base(BaseType::EnumType(name)) = etype.expr() {
                    self.add_instr(Instruction::ETest(f.clone(),name.to_string(),r.clone(),x),defstore)?;
                } else {
                    return Err("Can only take \"query\" of enums".to_string());
                }
                r
            },
            Expression::Pling(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                let etype = self.types.get_typeinf().get_sig(&Referrer::Register(x.clone())).clone();
                if let TypeSigExpr::Base(BaseType::EnumType(name)) = etype.expr() {
                    self.add_instr(Instruction::EValue(f.clone(),name.to_string(),r.clone(),x),defstore)?;
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
                r
            },
            Expression::Square(x) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                self.add_instr(Instruction::Square(r.clone(),x),defstore)?;
                r
            },
            Expression::Star(x) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                self.add_instr(Instruction::Star(r.clone(),x),defstore)?;
                r
            },
            Expression::Filter(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                let f = self.build_rvalue(defstore,f,&Some(x.clone()))?;
                self.add_instr(Instruction::Filter(r.clone(),x,f),defstore)?;
                r
            },
            Expression::Bracket(x,f) => {
                let r = self.regalloc.allocate();
                let x = self.build_rvalue(defstore,x,dollar)?;
                let xsq = self.regalloc.allocate();
                self.add_instr(Instruction::Square(xsq.clone(),x),defstore)?;
                let f = self.build_rvalue(defstore,f,&Some(xsq.clone()))?;
                self.add_instr(Instruction::Filter(r.clone(),xsq,f),defstore)?;
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
                    self.add_instr(Instruction::At(r.clone(),dollar.clone()),defstore)?;
                    r
                } else {
                    return Err("Unexpected $".to_string());
                }
            }
        })
    }
    // TODO deduplicate at

    fn build_stmt(&mut self, defstore: &DefStore, stmt: &Statement) -> Result<(),String> {
        let procdecl = defstore.get_proc(&stmt.0);
        if procdecl.is_none() {
            return Err(format!("No such procedure '{}'",stmt.0));
        }
        let lvalues : Vec<bool> = procdecl.unwrap().sigs().iter().map(|x| x.lvalue.is_some()).collect();
        let regs : Vec<Register> = self.map_expressions_top(defstore,&stmt.1,&lvalues)?;
        self.add_instr(Instruction::Proc(stmt.0.to_string(),regs.clone()),defstore)?;
        self.map_lvalues_top(defstore,&stmt.1,&regs,&lvalues)?;
        Ok(())
    }

    pub fn go(mut self, defstore: &DefStore, stmts: Vec<Statement>) -> Result<Vec<Instruction>,Vec<String>> {
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
    fn generate_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/generate-smoke2.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let regalloc = RegisterAllocator::new();
        let gen = Generator::new(&regalloc);
        let cmds : Vec<String> = gen.go(&defstore,stmts).expect("codegen").iter().map(|e| format!("{:?}",e)).collect();
        let outdata = load_testdata(&["codegen","generate-smoke2.out"]).ok().unwrap();
        assert_eq!(outdata,cmds.join(""));
    }
}