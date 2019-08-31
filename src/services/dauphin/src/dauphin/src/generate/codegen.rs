use std::collections::HashMap;
use std::collections::hash_map::Entry;

use super::intstruction2::Instruction2;
use crate::parser::{ Expression, Statement };
use crate::codegen::{ Register2, RegisterAllocator };
use crate::codegen::DefStore;
use crate::typeinf::{ BaseType, ExpressionType, Route, RouteExpr, SignatureMemberConstraint, Typing };

pub struct CodeGen {
    instrs: Vec<Instruction2>,
    regalloc: RegisterAllocator,
    regnames: HashMap<String,Register2>,
    typing: Typing,
    route: Route
}

impl CodeGen {
    pub fn new() -> CodeGen {
        CodeGen {
            instrs: Vec::new(),
            regalloc: RegisterAllocator::new(),
            regnames: HashMap::new(),
            typing: Typing::new(),
            route: Route::new()
        }
    }

    fn add_instr(&mut self, instr: Instruction2, defstore: &DefStore) -> Result<(),String> {
        print!("add_instr({:?})\n",instr);
        self.typing.add(&instr.get_constraint(defstore)?)?;
        self.instrs.push(instr);
        Ok(())
    }

    fn build_vec(&mut self, defstore: &DefStore, values: &Vec<Expression>, reg: &Register2, dollar: Option<&Register2>) -> Result<(),String> {
        self.add_instr(Instruction2::List(reg.clone()),defstore)?;
        for val in values {
            let r = self.regalloc.allocate2();
            self.build_rvalue(defstore,val,&r,dollar)?;
            let push = Instruction2::Push(reg.clone(),r.clone());
            self.add_instr(push,defstore)?;
        }
        Ok(())
    }

    fn struct_rearrange(&mut self, defstore: &DefStore, s: &str, x: Vec<Register2>, got_names: &Vec<String>) -> Result<Vec<Register2>,String> {
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

    fn type_of(&mut self, defstore: &DefStore, expr: &Expression) -> Result<ExpressionType,String> {
        Ok(match expr {
            Expression::Identifier(id) => {
                if !self.regnames.contains_key(id) {
                    return Err(format!("No such variable {:?}",id));
                }
                self.typing.get(&self.regnames[id])
            },
            Expression::Dot(x,f) => {
                if let ExpressionType::Base(BaseType::StructType(name)) = self.type_of(defstore,x)? {
                    if let Some(struct_) = defstore.get_struct(&name) {
                        if let Some(type_) = struct_.get_member_type2(f) {
                            type_.to_expressiontype()
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
            Expression::Pling(x,f) => {
                if let ExpressionType::Base(BaseType::EnumType(name)) = self.type_of(defstore,x)? {
                    if let Some(enum_) = defstore.get_enum(&name) {
                        if let Some(type_) = enum_.get_branch_type2(f) {
                            type_.to_expressiontype()
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
            Expression::Square(x) => {
                if let ExpressionType::Vec(subtype) = self.type_of(defstore,x)? {
                    subtype.as_ref().clone()
                } else {
                    return Err(format!("{:?} is not a vector",expr));
                }
            },
            Expression::Filter(x,f) => {
                self.type_of(defstore,x)?
            },
            _ => return Err(format!("Cannot type {:?}",expr))
        })
    }

    fn reg_nonref(&mut self, defstore: &DefStore, reg: &Register2) -> Result<Register2,String> {
        let (origin, exprs) = self.route.get(reg).clone();
        let mut reg = origin.clone();
        for expr in exprs.iter() {
            let subreg = self.regalloc.allocate2();
            match expr {
                RouteExpr::Filter(f) => {
                    self.add_instr(Instruction2::Filter(subreg.clone(),reg.clone(),f.clone()),defstore)?;
                },
                RouteExpr::Member(f) => {
                    let instr = match self.typing.get(&reg) {
                        ExpressionType::Base(BaseType::StructType(name)) =>
                            Instruction2::SValue(f.clone(),name.clone(),subreg.clone(),reg.clone()),
                        ExpressionType::Base(BaseType::EnumType(name)) =>
                            Instruction2::EValue(f.clone(),name.clone(),subreg.clone(),reg.clone()),
                        _ => return Err(format!("unexpected type\n"))
                    };
                    self.add_instr(instr,defstore)?;
                },
                RouteExpr::Square => {
                    self.add_instr(Instruction2::Square(subreg.clone(),reg.clone()),defstore)?;
                }
            }
            reg = subreg;
        }
        Ok(reg)
    }

    fn build_lvalue(&mut self, defstore: &DefStore, expr: &Expression, reg: &Register2, top: bool) -> Result<(),String> {
        match expr {
            Expression::Identifier(id) => {
                if top {
                    // if it's a top level assignment allow type change
                    self.regnames.remove(id);
                }
                if !self.regnames.contains_key(id) {
                    self.regnames.insert(id.clone(),self.regalloc.allocate2());
                }
                let real_reg = self.regnames[id].clone();
                self.add_instr(Instruction2::Ref(reg.clone(),real_reg.clone()),defstore)?;
                self.route.set_empty(&reg,&real_reg);
            },
            Expression::Dot(x,f) => {
                if let ExpressionType::Base(BaseType::StructType(name)) = self.type_of(defstore,x)? {
                    let subreg = self.regalloc.allocate2();
                    self.build_lvalue(defstore,x,&subreg,false)?;
                    self.add_instr(Instruction2::RefSValue(f.clone(),name.to_string(),reg.clone(),subreg.clone()),defstore)?;
                    self.route.set_derive(&reg,&subreg,&RouteExpr::Member(f.to_string()));
                    
                } else {
                    return Err("Can only take \"dot\" of structs".to_string());
                }
            },
            Expression::Pling(x,f) => {
                if let ExpressionType::Base(BaseType::EnumType(name)) = self.type_of(defstore,x)? {
                    let subreg = self.regalloc.allocate2();
                    self.build_lvalue(defstore,x,&subreg,false)?;
                    self.add_instr(Instruction2::RefEValue(f.clone(),name.to_string(),reg.clone(),subreg.clone()),defstore)?;
                    self.route.set_derive(&reg,&subreg,&RouteExpr::Member(f.to_string()));
                    
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
            },
            Expression::Square(x) => {
                let subreg = self.regalloc.allocate2();
                self.build_lvalue(defstore,x,&subreg,false)?;
                self.add_instr(Instruction2::RefSquare(reg.clone(),subreg.clone()),defstore)?;
                self.route.set_derive(&reg,&subreg,&RouteExpr::Square);                
            },
            Expression::Filter(x,f) => {
                let subreg = self.regalloc.allocate2();
                self.build_lvalue(defstore,x,&subreg,false)?;
                let filterreg = self.regalloc.allocate2();
                let argreg = self.reg_nonref(defstore,&subreg)?;
                self.build_rvalue(defstore,f,&filterreg,Some(&argreg))?;
                self.add_instr(Instruction2::RefFilter(reg.clone(),subreg.clone(),filterreg.clone()),defstore)?;
                /* make permanent copy of filterreg to avoid competing updates */
                let permreg = self.regalloc.allocate2();
                self.add_instr(Instruction2::Copy(permreg.clone(),filterreg.clone()),defstore)?;
                self.route.set_derive(&subreg,&subreg,&RouteExpr::Filter(permreg));
            },
            Expression::Bracket(x,f) => {
                let interreg = self.regalloc.allocate2();
                let subreg = self.regalloc.allocate2();
                self.build_lvalue(defstore,x,&subreg,false)?;
                self.add_instr(Instruction2::RefSquare(interreg.clone(),subreg.clone()),defstore)?;
                self.route.set_derive(&interreg,&subreg,&RouteExpr::Square);
                let filterreg = self.regalloc.allocate2();
                let argreg = self.reg_nonref(defstore,&interreg)?;
                self.build_rvalue(defstore,f,&filterreg,Some(&argreg))?;
                self.add_instr(Instruction2::RefFilter(reg.clone(),interreg.clone(),filterreg.clone()),defstore)?;
                /* make permanent copy of filterreg to avoid competing updates */
                let permreg = self.regalloc.allocate2();
                self.add_instr(Instruction2::Copy(permreg.clone(),filterreg.clone()),defstore)?;
                self.route.set_derive(&reg,&interreg,&RouteExpr::Filter(permreg));
            },
            _ => return Err("Invalid lvalue".to_string())
        }
        Ok(())
    }

    fn build_rvalue(&mut self, defstore: &DefStore, expr: &Expression, reg: &Register2, dollar: Option<&Register2>) -> Result<(),String> {
        match expr {
            Expression::Identifier(id) => {
                if !self.regnames.contains_key(id) {
                    return Err(format!("Unset variable {:?}",id));
                }
                let real_reg = self.regnames[id].clone();
                self.add_instr(Instruction2::Copy(reg.clone(),real_reg),defstore)?;
            },
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
            Expression::Vector(v) => self.build_vec(defstore,v,reg,dollar)?,
            Expression::Operator(name,x) => {
                let mut subregs = Vec::new();
                for e in x {
                    let r = self.regalloc.allocate2();
                    self.build_rvalue(defstore,e,&r,dollar)?;
                    subregs.push(r);
                }
                self.add_instr(Instruction2::Operator(name.clone(),reg.clone(),subregs),defstore)?;
            },
            Expression::CtorStruct(s,x,n) => {
                let mut subregs = Vec::new();
                for e in x {
                    let r = self.regalloc.allocate2();
                    self.build_rvalue(defstore,e,&r,dollar)?;
                    subregs.push(r);
                }
                let x = self.struct_rearrange(defstore,s,subregs,n)?;
                self.add_instr(Instruction2::CtorStruct(s.clone(),reg.clone(),x),defstore)?;
            },
            Expression::CtorEnum(e,b,x) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.add_instr(Instruction2::CtorEnum(e.clone(),b.clone(),reg.clone(),subreg),defstore)?;
            },
            Expression::Dot(x,f) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let stype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::StructType(name)) = stype {
                    self.add_instr(Instruction2::SValue(f.clone(),name.to_string(),reg.clone(),subreg),defstore)?;
                } else {
                    return Err(format!("Can only take \"dot\" of structs, not {:?}",stype));
                }
            },
            Expression::Query(x,f) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let etype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    self.add_instr(Instruction2::ETest(f.clone(),name.to_string(),reg.clone(),subreg),defstore)?;
                } else {
                    return Err("Can only take \"query\" of enums".to_string());
                }
            },
            Expression::Pling(x,f) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let etype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    self.add_instr(Instruction2::EValue(f.clone(),name.to_string(),reg.clone(),subreg),defstore)?;
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
            },
            Expression::Square(x) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.add_instr(Instruction2::Square(reg.clone(),subreg),defstore)?;
            },
            Expression::Star(x) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.add_instr(Instruction2::Star(reg.clone(),subreg),defstore)?;
            },
            Expression::Filter(x,f) => {
                let subreg = self.regalloc.allocate2();
                let filterreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.build_rvalue(defstore,f,&filterreg,Some(&subreg))?;
                self.add_instr(Instruction2::Filter(reg.clone(),subreg.clone(),filterreg.clone()),defstore)?;
            },
            Expression::Bracket(x,f) => {
                let subreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let sq_subreg = self.regalloc.allocate2();
                self.add_instr(Instruction2::Square(sq_subreg.clone(),subreg.clone()),defstore)?;
                let filterreg = self.regalloc.allocate2();
                self.build_rvalue(defstore,f,&filterreg,Some(&sq_subreg))?;
                self.add_instr(Instruction2::Filter(reg.clone(),sq_subreg.clone(),filterreg.clone()),defstore)?;
            },
            Expression::Dollar => {
                if let Some(dollar) = dollar {
                    self.add_instr(Instruction2::Copy(reg.clone(),dollar.clone()),defstore)?;
                } else {
                    return Err("Unexpected $".to_string());
                }
            },
            Expression::At => {
                if let Some(dollar) = dollar {
                    self.add_instr(Instruction2::At(reg.clone(),dollar.clone()),defstore)?;
                } else {
                    return Err("Unexpected $".to_string());
                }
            }
        };
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
            match member {
                SignatureMemberConstraint::RValue(_) =>
                    self.build_rvalue(defstore,&stmt.1[i],&regs[i],None)?,
                SignatureMemberConstraint::LValue(_) =>
                    self.build_lvalue(defstore,&stmt.1[i],&regs[i],true)?,
            }
        }
        self.add_instr(Instruction2::Proc(stmt.0.to_string(),regs.clone()),defstore)?;
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
        print!("{:?}\n",self.typing);
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

    fn run_pass(filename: &str) -> Result<(),Vec<String>> {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("test:codegen/{}",filename)).expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let gen = CodeGen::new();
        gen.go(&defstore,stmts)?;
        Ok(())
    }

    #[test]
    fn codegen_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/generate-smoke2.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let gen = CodeGen::new();
        let cmds : Vec<String> = gen.go(&defstore,stmts).expect("codegen").iter().map(|e| format!("{:?}",e)).collect();
        let outdata = load_testdata(&["codegen","generate-smoke2.out"]).ok().unwrap();
        assert_eq!(outdata,cmds.join(""));
    }

    #[test]
    fn codegen_lvalue_checks() {
        run_pass("typepass-reassignok.dp").expect("A");
        run_pass("typepass-reassignbad.dp").expect_err("B");
    }
}
