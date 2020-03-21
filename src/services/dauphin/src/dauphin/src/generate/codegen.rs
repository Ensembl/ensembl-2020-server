use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use super::intstruction::Instruction;
use crate::opcodes::{ CtorEnum, CtorStruct, SValue, EValue, ETest };
use crate::parser::{ Expression, Statement };
use crate::model::{ Register, RegisterAllocator };
use crate::model::DefStore;
use crate::typeinf::{ BaseType, ExpressionType, SignatureMemberConstraint, TypeModel, Typing, MemberMode };

pub struct GenContext {
    pub instrs: Vec<Instruction>,
    pub regalloc: RegisterAllocator,
    pub types: TypeModel
}

impl fmt::Debug for GenContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instr_str : Vec<String> = self.instrs.iter().map(|v| format!("{:?}",v)).collect();
        write!(f,"{:?}\n{}\n",self.types,instr_str.join(""))?;
        Ok(())
    }
}

pub struct CodeGen<'a> {
    context: GenContext,
    defstore: &'a DefStore,
    typing: Typing,
    regnames: HashMap<String,Register>
}

impl<'a> CodeGen<'a> {
    fn new(defstore: &'a DefStore) -> CodeGen {
        CodeGen {
            context: GenContext {
                instrs: Vec::new(),
                regalloc: RegisterAllocator::new(),
                types: TypeModel::new()
            },
            defstore,
            typing: Typing::new(),
            regnames: HashMap::new()
        }
    }

    fn add_instr(&mut self, instr: Instruction) -> Result<(),String> {
        self.typing.add(&instr.get_constraint(self.defstore)?)?;
        self.context.instrs.push(instr);
        Ok(())
    }

    fn build_vec(&mut self, values: &Vec<Expression>, reg: Register, dollar: Option<&Register>, at: Option<&Register>) -> Result<(),String> {
        let tmp = self.context.regalloc.allocate();
        self.add_instr(Instruction::Nil(tmp))?;
        for val in values {
            let r = self.build_rvalue(val,dollar,at)?;
            let push = Instruction::Append(tmp,r);
            self.add_instr(push)?;
        }
        self.add_instr(Instruction::Star(reg,tmp))?;
        Ok(())
    }

    fn struct_rearrange(&mut self, s: &str, x: Vec<Register>, got_names: &Vec<String>) -> Result<Vec<Register>,String> {
        if let Some(decl) = self.defstore.get_struct(s) {
            let gotpos : HashMap<String,usize> = got_names.iter().enumerate().map(|(i,e)| (e.to_string(),i)).collect();
            let mut out = Vec::new();
            for want_name in decl.get_names().iter() {
                if let Some(got_pos) = gotpos.get(want_name) {
                    out.push(x[*got_pos]);
                } else {
                    return Err(format!("Missing member '{}'",want_name));
                }
            }
            Ok(out)
        } else {
            Err(format!("no such struct '{}'",s))
        }
    }

    fn type_of(&mut self, expr: &Expression) -> Result<ExpressionType,String> {
        Ok(match expr {
            Expression::Identifier(id) => {
                if !self.regnames.contains_key(id) {
                    return Err(format!("No such variable {:?}",id));
                }
                self.typing.get(&self.regnames[id])
            },
            Expression::Dot(x,f) => {
                if let ExpressionType::Base(BaseType::StructType(name)) = self.type_of(x)? {
                    if let Some(struct_) = self.defstore.get_struct(&name) {
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
                if let ExpressionType::Base(BaseType::EnumType(name)) = self.type_of(x)? {
                    if let Some(enum_) = self.defstore.get_enum(&name) {
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
                if let ExpressionType::Vec(subtype) = self.type_of(x)? {
                    subtype.as_ref().clone()
                } else {
                    return Err(format!("{:?} is not a vector",expr));
                }
            },
            Expression::Filter(x,_) => {
                self.type_of(x)?
            },
            _ => return Err(format!("Cannot type {:?}",expr))
        })
    }

    fn build_lvalue(&mut self, expr: &Expression, top: bool, unfiltered_in: bool) -> Result<(Register,Option<Register>,Register),String> {
        match expr {
            Expression::Identifier(id) => {
                if top {
                    // if it's a top level assignment allow type change
                    self.regnames.remove(id);
                }
                if !self.regnames.contains_key(id) {
                    self.regnames.insert(id.clone(),self.context.regalloc.allocate());
                }
                let real_reg = self.regnames[id];
                let lvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::LValue(lvalue_reg,real_reg))?;
                Ok((lvalue_reg,None,real_reg))
            },
            Expression::Dot(x,f) => {
                if let ExpressionType::Base(BaseType::StructType(name)) = self.type_of(x)? {
                    let (lvalue_subreg,fvalue_reg,rvalue_subreg) = self.build_lvalue(x,false,unfiltered_in)?;
                    let lvalue_reg = self.context.regalloc.allocate();
                    let rvalue_reg = self.context.regalloc.allocate();
                    self.add_instr(Instruction::New(Rc::new(SValue::new(name.to_string(),f.clone(),lvalue_reg,lvalue_subreg))))?;
                    self.add_instr(Instruction::New(Rc::new(SValue::new(name.to_string(),f.clone(),rvalue_reg,rvalue_subreg))))?;
                    Ok((lvalue_reg,fvalue_reg,rvalue_reg))
                } else {
                    Err("Can only take \"dot\" of structs".to_string())
                }
            },
            Expression::Pling(x,f) => {
                if let ExpressionType::Base(BaseType::EnumType(name)) = self.type_of(x)? {
                    let (lvalue_subreg,fvalue_reg,rvalue_subreg) = self.build_lvalue(x,false,unfiltered_in)?;
                    let lvalue_reg = self.context.regalloc.allocate();
                    let rvalue_reg = self.context.regalloc.allocate();
                    self.add_instr(Instruction::New(Rc::new(EValue::new(name.to_string(),f.clone(),lvalue_reg,lvalue_subreg))))?;
                    self.add_instr(Instruction::New(Rc::new(EValue::new(name.to_string(),f.clone(),rvalue_reg,rvalue_subreg))))?;
                    Ok((lvalue_reg,fvalue_reg,rvalue_reg))
                } else {
                    Err("Can only take \"pling\" of enums".to_string())
                }
            },
            Expression::Square(x) => {
                let (lvalue_subreg,_,rvalue_subreg) = self.build_lvalue(x,false,false)?;
                let lvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::RefSquare(lvalue_reg,lvalue_subreg))?;
                let rvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Square(rvalue_reg,rvalue_subreg))?;
                let fvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::FilterSquare(fvalue_reg,rvalue_subreg))?;
                Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
            },
            Expression::Filter(x,f) => {
                let (lvalue_reg,fvalue_subreg,rvalue_subreg) = self.build_lvalue(x,false,false)?;
                /* Unlike in a bracket, @ makes no sense in a filter as the array has already been lost */
                let filterreg = self.build_rvalue(f,Some(&rvalue_subreg),None)?;                
                let fvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Filter(fvalue_reg,fvalue_subreg.unwrap(),filterreg))?;
                let rvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Filter(rvalue_reg,rvalue_subreg,filterreg))?;
                Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
            },
            Expression::Bracket(x,f) => {
                let (lvalue_subreg,_,rvalue_subreg) = self.build_lvalue(x,false,false)?;
                let lvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::RefSquare(lvalue_reg,lvalue_subreg))?;
                let rvalue_interreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Square(rvalue_interreg,rvalue_subreg))?;
                let fvalue_interreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::FilterSquare(fvalue_interreg,rvalue_subreg))?;
                let atreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::At(atreg,rvalue_subreg))?;
                let filterreg = self.build_rvalue(f,Some(&rvalue_interreg),Some(&atreg))?;
                let fvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Filter(fvalue_reg,fvalue_interreg,filterreg))?;
                let rvalue_reg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Filter(rvalue_reg,rvalue_interreg,filterreg))?;
                Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
            },
            _ => return Err("Invalid lvalue".to_string())
        }
    }

    fn build_rvalue(&mut self, expr: &Expression, dollar: Option<&Register>, at: Option<&Register>) -> Result<Register,String> {
        let reg = self.context.regalloc.allocate();
        match expr {
            Expression::Identifier(id) => {
                if !self.regnames.contains_key(id) {
                    return Err(format!("Unset variable {:?}",id));
                }
                let real_reg = self.regnames[id];
                self.add_instr(Instruction::Copy(reg,real_reg))?;
            },
            Expression::Number(n) => {
                self.add_instr(Instruction::NumberConst(reg,*n))?;
            },
            Expression::LiteralString(s) => {
                self.add_instr(Instruction::StringConst(reg,s.to_string()))?;
            },
            Expression::LiteralBool(b) => {
                self.add_instr(Instruction::BooleanConst(reg,*b))?;
            },
            Expression::LiteralBytes(b) => {
                self.add_instr(Instruction::BytesConst(reg,b.to_vec()))?;
            },
            Expression::Vector(v) => self.build_vec(v,reg,dollar,at)?,
            Expression::Operator(name,x) => {
                let mut subregs = Vec::new();
                for e in x {
                    let r = self.build_rvalue(e,dollar,at)?;
                    subregs.push(r);
                }
                self.add_instr(Instruction::Operator(name.clone(),vec![reg],subregs))?;
            },
            Expression::CtorStruct(s,x,n) => {
                let mut subregs = Vec::new();
                for e in x {
                    let r = self.build_rvalue(e,dollar,at)?;
                    subregs.push(r);
                }
                let x = self.struct_rearrange(s,subregs,n)?;
                self.add_instr(Instruction::New(Rc::new(CtorStruct::new(s.clone(),reg,x))))?;
            },
            Expression::CtorEnum(e,b,x) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                self.add_instr(Instruction::New(Rc::new(CtorEnum::new(e.clone(),b.clone(),reg,subreg))))?;
            },
            Expression::Dot(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let stype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::StructType(name)) = stype {
                    self.add_instr(Instruction::New(Rc::new(SValue::new(name.to_string(),f.clone(),reg,subreg))))?;
                } else {
                    return Err(format!("Can only take \"dot\" of structs, not {:?}",stype));
                }
            },
            Expression::Query(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let etype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    self.add_instr(Instruction::New(Rc::new(ETest::new(name.to_string(),f.clone(),reg,subreg))))?;
                } else {
                    return Err("Can only take \"query\" of enums".to_string());
                }
            },
            Expression::Pling(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let etype = self.typing.get(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    self.add_instr(Instruction::New(Rc::new(EValue::new(name.clone(),f.to_string(),reg,subreg))))?;
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
            },
            Expression::Square(x) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                self.add_instr(Instruction::Square(reg,subreg))?;
            },
            Expression::Star(x) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                self.add_instr(Instruction::Star(reg,subreg))?;
            },
            Expression::Filter(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                /* Unlike in a bracket, @ makes no sense in a filter as the array has already been lost */
                let filterreg = self.build_rvalue(f,Some(&subreg),None)?;
                self.add_instr(Instruction::Filter(reg,subreg,filterreg))?;
            },
            Expression::Bracket(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let atreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::At(atreg,subreg))?;
                let sq_subreg = self.context.regalloc.allocate();
                self.add_instr(Instruction::Square(sq_subreg,subreg))?;
                let filterreg = self.build_rvalue(f,Some(&sq_subreg),Some(&atreg))?;
                self.add_instr(Instruction::Filter(reg,sq_subreg,filterreg))?;
            },
            Expression::Dollar => {
                if let Some(dollar) = dollar {
                    self.add_instr(Instruction::Copy(reg,*dollar))?;
                } else {
                    return Err("Unexpected $".to_string());
                }
            },
            Expression::At => {
                if let Some(at) = at {
                    self.add_instr(Instruction::Copy(reg,*at))?;
                } else {
                    return Err("Unexpected @".to_string());
                }
            }
        };
        Ok(reg)
    }

    fn build_stmt(&mut self, stmt: &Statement) -> Result<(),String> {
        let mut regs = Vec::new();
        let procdecl = self.defstore.get_proc(&stmt.0);
        if procdecl.is_none() {
            return Err(format!("No such procedure '{}'",stmt.0));
        }
        for (i,member) in procdecl.unwrap().get_signature().each_member().enumerate() {
            match member {
                SignatureMemberConstraint::RValue(_) =>
                    regs.push((MemberMode::RValue,self.build_rvalue(&stmt.1[i],None,None)?)),
                SignatureMemberConstraint::LValue(_) => {
                    let (lvalue_reg,fvalue_reg,_) = self.build_lvalue(&stmt.1[i],true,true)?;
                    if let Some(fvalue_reg) = fvalue_reg {
                        regs.push((MemberMode::FValue,fvalue_reg));
                    }
                    regs.push((MemberMode::LValue,lvalue_reg));
                }
            }
        }
        self.add_instr(Instruction::Proc(stmt.0.to_string(),regs.clone()))?;
        Ok(())
    }

    fn go(mut self, stmts: Vec<Statement>) -> Result<GenContext,Vec<String>> {
        let mut errors = Vec::new();
        for stmt in &stmts {
            let r = self.build_stmt(stmt);
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

pub fn generate_code<'a>(defstore: &'a DefStore, stmts: Vec<Statement>) -> Result<GenContext,Vec<String>> {
    CodeGen::new(defstore).go(stmts)
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
        let gen = CodeGen::new(&defstore);
        gen.go(stmts)?;
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
