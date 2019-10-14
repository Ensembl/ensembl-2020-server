use std::collections::HashMap;
use std::fmt;

use super::intstruction::Instruction;
use crate::parser::{ Expression, Statement };
use crate::model::{ Register, RegisterAllocator };
use crate::model::DefStore;
use crate::typeinf::{ BaseType, ExpressionType, Route, RouteExpr, SignatureMemberConstraint, TypeModel, Typing };

pub struct GenContext {
    pub instrs: Vec<Instruction>,
    pub regalloc: RegisterAllocator,
    pub route: Route,
    pub types: TypeModel
}

impl fmt::Debug for GenContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instr_str : Vec<String> = self.instrs.iter().map(|v| format!("{:?}",v)).collect();
        write!(f,"{:?}\n{:?}{}\n",self.types,self.route,instr_str.join(""))?;
        Ok(())
    }
}

pub struct CodeGen {
    context: GenContext,
    typing: Typing,
    regnames: HashMap<String,Register>
}

impl CodeGen {
    fn new() -> CodeGen {
        CodeGen {
            context: GenContext {
                instrs: Vec::new(),
                regalloc: RegisterAllocator::new(),
                route: Route::new(),
                types: TypeModel::new()
            },
            typing: Typing::new(),
            regnames: HashMap::new()
        }
    }

    fn add_instr(&mut self, instr: Instruction, defstore: &DefStore) -> Result<(),String> {
        self.typing.add(&instr.get_constraint(defstore)?)?;
        self.context.instrs.push(instr);
        Ok(())
    }

    fn build_vec(&mut self, defstore: &DefStore, values: &Vec<Expression>, reg: &Register, dollar: Option<&Register>) -> Result<(),String> {
        let tmp = self.context.regalloc.allocate();
        self.add_instr(Instruction::Nil(tmp.clone()),defstore)?;
        for val in values {
            let r = self.context.regalloc.allocate();
            self.build_rvalue(defstore,val,&r,dollar)?;
            let push = Instruction::Append(tmp.clone(),r.clone());
            self.add_instr(push,defstore)?;
        }
        self.add_instr(Instruction::Star(reg.clone(),tmp),defstore)?;
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
                        if let Some(type_) = struct_.get_member_type(f) {
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
                        if let Some(type_) = enum_.get_branch_type(f) {
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
            Expression::Filter(x,_) => {
                self.type_of(defstore,x)?
            },
            _ => return Err(format!("Cannot type {:?}",expr))
        })
    }

    fn reg_nonref(&mut self, defstore: &DefStore, reg: &Register) -> Result<Register,String> {
        let (origin, exprs) = self.context.route.get(reg).ok_or_else(|| format!("cannot find register")).unwrap().clone();
        let mut reg = origin.clone();
        for expr in exprs.iter() {
            let subreg = self.context.regalloc.allocate();
            match expr {
                RouteExpr::Filter(f) => {
                    self.add_instr(Instruction::Filter(subreg.clone(),reg.clone(),f.clone()),defstore)?;
                },
                RouteExpr::Member(f) => {
                    let instr = match self.typing.get(&reg) {
                        ExpressionType::Base(BaseType::StructType(name)) =>
                            Instruction::SValue(f.clone(),name.clone(),subreg.clone(),reg.clone()),
                        ExpressionType::Base(BaseType::EnumType(name)) =>
                            Instruction::EValue(f.clone(),name.clone(),subreg.clone(),reg.clone()),
                        _ => return Err(format!("unexpected type\n"))
                    };
                    self.add_instr(instr,defstore)?;
                },
                RouteExpr::Square => {
                    self.add_instr(Instruction::Square(subreg.clone(),reg.clone()),defstore)?;
                }
            }
            reg = subreg;
        }
        Ok(reg)
    }

    fn build_lvalue(&mut self, defstore: &DefStore, expr: &Expression, reg: &Register, top: bool) -> Result<(),String> {
        match expr {
            Expression::Identifier(id) => {
                if top {
                    // if it's a top level assignment allow type change
                    self.regnames.remove(id);
                }
                if !self.regnames.contains_key(id) {
                    self.regnames.insert(id.clone(),self.context.regalloc.allocate());
                }
                let real_reg = self.regnames[id].clone();
                self.add_instr(Instruction::Ref(reg.clone(),real_reg.clone()),defstore)?;
                self.context.route.set_empty(&reg,&real_reg);
            },
            Expression::Dot(x,f) => {
                if let ExpressionType::Base(BaseType::StructType(name)) = self.type_of(defstore,x)? {
                    let subreg = self.context.regalloc.allocate();
                    self.build_lvalue(defstore,x,&subreg,false)?;
                    self.add_instr(Instruction::RefSValue(f.clone(),name.to_string(),reg.clone(),subreg.clone()),defstore)?;
                    self.context.route.set_derive(&reg,&subreg,&RouteExpr::Member(f.to_string()));
                    
                } else {
                    return Err("Can only take \"dot\" of structs".to_string());
                }
            },
            Expression::Pling(x,f) => {
                if let ExpressionType::Base(BaseType::EnumType(name)) = self.type_of(defstore,x)? {
                    let subreg = self.context.regalloc.allocate();
                    self.build_lvalue(defstore,x,&subreg,false)?;
                    self.add_instr(Instruction::RefEValue(f.clone(),name.to_string(),reg.clone(),subreg.clone()),defstore)?;
                    self.context.route.set_derive(&reg,&subreg,&RouteExpr::Member(f.to_string()));
                    
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
            },
            Expression::Square(x) => {
                let subreg = self.context.regalloc.allocate();
                self.build_lvalue(defstore,x,&subreg,false)?;
                self.add_instr(Instruction::RefSquare(reg.clone(),subreg.clone()),defstore)?;
                self.context.route.set_derive(&reg,&subreg,&RouteExpr::Square);                
            },
            Expression::Filter(x,f) => {
                let subreg = self.context.regalloc.allocate();
                self.build_lvalue(defstore,x,&subreg,false)?;
                let filterreg = self.context.regalloc.allocate();
                let argreg = self.reg_nonref(defstore,&subreg)?;
                self.build_rvalue(defstore,f,&filterreg,Some(&argreg))?;
                self.add_instr(Instruction::RefFilter(reg.clone(),subreg.clone(),filterreg.clone()),defstore)?;
                /* make permanent copy of filterreg to avoid competing updates */
                let permreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Copy(permreg.clone(),filterreg.clone()),defstore)?;
                self.context.route.set_derive(&subreg,&subreg,&RouteExpr::Filter(permreg));
            },
            Expression::Bracket(x,f) => {
                let interreg = self.context.regalloc.allocate();
                let subreg = self.context.regalloc.allocate();
                self.build_lvalue(defstore,x,&subreg,false)?;
                self.add_instr(Instruction::RefSquare(interreg.clone(),subreg.clone()),defstore)?;
                self.context.route.set_derive(&interreg,&subreg,&RouteExpr::Square);
                let filterreg = self.context.regalloc.allocate();
                let argreg = self.reg_nonref(defstore,&interreg)?;
                self.build_rvalue(defstore,f,&filterreg,Some(&argreg))?;
                /* make permanent copy of filterreg to avoid competing updates */
                let permreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Copy(permreg.clone(),filterreg.clone()),defstore)?;
                self.add_instr(Instruction::RefFilter(reg.clone(),interreg.clone(),permreg.clone()),defstore)?;
                self.context.route.set_derive(&reg,&interreg,&RouteExpr::Filter(permreg));
            },
            _ => return Err("Invalid lvalue".to_string())
        }
        Ok(())
    }

    fn build_rvalue(&mut self, defstore: &DefStore, expr: &Expression, reg: &Register, dollar: Option<&Register>) -> Result<(),String> {
        match expr {
            Expression::Identifier(id) => {
                if !self.regnames.contains_key(id) {
                    return Err(format!("Unset variable {:?}",id));
                }
                let real_reg = self.regnames[id].clone();
                self.add_instr(Instruction::Copy(reg.clone(),real_reg),defstore)?;
            },
            Expression::Number(n) => {
                self.add_instr(Instruction::NumberConst(reg.clone(),*n),defstore)?;
            },
            Expression::LiteralString(s) => {
                self.add_instr(Instruction::StringConst(reg.clone(),s.to_string()),defstore)?;
            },
            Expression::LiteralBool(b) => {
                self.add_instr(Instruction::BooleanConst(reg.clone(),*b),defstore)?;
            },
            Expression::LiteralBytes(b) => {
                self.add_instr(Instruction::BytesConst(reg.clone(),b.to_vec()),defstore)?;
            },
            Expression::Vector(v) => self.build_vec(defstore,v,reg,dollar)?,
            Expression::Operator(name,x) => {
                let mut subregs = Vec::new();
                for e in x {
                    let r = self.context.regalloc.allocate();
                    self.build_rvalue(defstore,e,&r,dollar)?;
                    subregs.push(r);
                }
                self.add_instr(Instruction::Operator(name.clone(),vec![reg.clone()],subregs),defstore)?;
            },
            Expression::CtorStruct(s,x,n) => {
                let mut subregs = Vec::new();
                for e in x {
                    let r = self.context.regalloc.allocate();
                    self.build_rvalue(defstore,e,&r,dollar)?;
                    subregs.push(r);
                }
                let x = self.struct_rearrange(defstore,s,subregs,n)?;
                self.add_instr(Instruction::CtorStruct(s.clone(),reg.clone(),x),defstore)?;
            },
            Expression::CtorEnum(e,b,x) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.add_instr(Instruction::CtorEnum(e.clone(),b.clone(),reg.clone(),subreg),defstore)?;
            },
            Expression::Dot(x,f) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let stype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::StructType(name)) = stype {
                    self.add_instr(Instruction::SValue(f.clone(),name.to_string(),reg.clone(),subreg),defstore)?;
                } else {
                    return Err(format!("Can only take \"dot\" of structs, not {:?}",stype));
                }
            },
            Expression::Query(x,f) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let etype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    self.add_instr(Instruction::ETest(f.clone(),name.to_string(),reg.clone(),subreg),defstore)?;
                } else {
                    return Err("Can only take \"query\" of enums".to_string());
                }
            },
            Expression::Pling(x,f) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let etype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    self.add_instr(Instruction::EValue(f.clone(),name.to_string(),reg.clone(),subreg),defstore)?;
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
            },
            Expression::Square(x) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.add_instr(Instruction::Square(reg.clone(),subreg),defstore)?;
            },
            Expression::Star(x) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.add_instr(Instruction::Star(reg.clone(),subreg),defstore)?;
            },
            Expression::Filter(x,f) => {
                let subreg = self.context.regalloc.allocate();
                let filterreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                self.build_rvalue(defstore,f,&filterreg,Some(&subreg))?;
                self.add_instr(Instruction::Filter(reg.clone(),subreg.clone(),filterreg.clone()),defstore)?;
            },
            Expression::Bracket(x,f) => {
                let subreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,x,&subreg,dollar)?;
                let sq_subreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Square(sq_subreg.clone(),subreg.clone()),defstore)?;
                let filterreg = self.context.regalloc.allocate();
                self.build_rvalue(defstore,f,&filterreg,Some(&sq_subreg))?;
                self.add_instr(Instruction::Filter(reg.clone(),sq_subreg.clone(),filterreg.clone()),defstore)?;
            },
            Expression::Dollar => {
                if let Some(dollar) = dollar {
                    self.add_instr(Instruction::Copy(reg.clone(),dollar.clone()),defstore)?;
                } else {
                    return Err("Unexpected $".to_string());
                }
            },
            Expression::At => {
                if let Some(dollar) = dollar {
                    self.add_instr(Instruction::At(reg.clone(),dollar.clone()),defstore)?;
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
        for _ in procdecl.unwrap().get_signature().each_member() {
            regs.push(self.context.regalloc.allocate());
        }
        for (i,member) in procdecl.unwrap().get_signature().each_member().enumerate() {
            match member {
                SignatureMemberConstraint::RValue(_) =>
                    self.build_rvalue(defstore,&stmt.1[i],&regs[i],None)?,
                SignatureMemberConstraint::LValue(_) =>
                    self.build_lvalue(defstore,&stmt.1[i],&regs[i],true)?,
            }
        }
        self.add_instr(Instruction::Proc(stmt.0.to_string(),regs.clone()),defstore)?;
        Ok(())
    }

    fn go(mut self, defstore: &DefStore, stmts: Vec<Statement>) -> Result<GenContext,Vec<String>> {
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
            self.typing.to_model(&mut self.context.types);
            Ok(self.context)
        }
    }
}

pub fn generate_code(defstore: &DefStore, stmts: Vec<Statement>) -> Result<GenContext,Vec<String>> {
    CodeGen::new().go(defstore,stmts)
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
        let cmds : Vec<String> = generate_code(&defstore,stmts).expect("codegen").instrs.iter().map(|e| format!("{:?}",e)).collect();
        let outdata = load_testdata(&["codegen","generate-smoke2.out"]).ok().unwrap();
        assert_eq!(outdata,cmds.join(""));
    }

    #[test]
    fn codegen_lvalue_checks() {
        run_pass("typepass-reassignok.dp").expect("A");
        run_pass("typepass-reassignbad.dp").expect_err("B");
    }
}
