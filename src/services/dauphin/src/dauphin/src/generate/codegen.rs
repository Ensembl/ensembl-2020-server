/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *  
 *  vscode-fold=1
 */

use std::collections::HashMap;

use super::gencontext::GenContext;
use super::instruction::{ Instruction, InstructionType };
use crate::parser::{ Expression, Statement };
use crate::model::Register;
use crate::model::DefStore;
use crate::typeinf::{ BaseType, ExpressionType, SignatureMemberConstraint, MemberMode };

macro_rules! addf {
    ($this:expr,$opcode:tt,$($regs:expr),*) => {
        $this.context.add_untyped_f(InstructionType::$opcode,vec![$($regs),*])?
    };
    ($this:expr,$opcode:tt($($args:expr),*),$($regs:expr),*) => {
        $this.context.add_untyped_f(InstructionType::$opcode($($args),*),vec![$($regs),*])?
    };
    ($this:expr,$opcode:tt) => {
        $this.context.add_untyped_f(InstructionType::$opcode,vec![])?
    };
    ($this:expr,$opcode:tt($($args:expr),*)) => {
        $this.context.add_untyped_f(InstructionType::$opcode($($args),*),vec![])?
    };
}

pub struct CodeGen<'a> {
    context: GenContext<'a>,
    defstore: &'a DefStore,
    regnames: HashMap<String,Register>
}

impl<'a> CodeGen<'a> {
    fn new(defstore: &'a DefStore) -> CodeGen {
        CodeGen {
            context: GenContext::new(defstore),
            defstore,
            regnames: HashMap::new()
        }
    }

    fn build_vec(&mut self, values: &Vec<Expression>, dollar: Option<&Register>, at: Option<&Register>) -> Result<Register,String> {
        let tmp = addf!(self,Nil);
        for val in values {
            let r = self.build_rvalue(val,dollar,at)?;
            self.context.add_untyped(Instruction::new(InstructionType::Append,vec![tmp,r]))?;
        }
        Ok(addf!(self,Star,tmp))

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
                self.context.get_partial_type(&self.regnames[id])
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
            Expression::Square(x) | Expression::Bracket(x,_) => {
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
                    self.regnames.insert(id.clone(),self.context.allocate_register(None));
                }
                let real_reg = self.regnames[id];
                let lvalue_reg = addf!(self,Alias,real_reg);
                Ok((lvalue_reg,None,real_reg))
            },
            Expression::Dot(x,f) => {
                if let ExpressionType::Base(BaseType::StructType(name)) = self.type_of(x)? {
                    let (lvalue_subreg,fvalue_reg,rvalue_subreg) = self.build_lvalue(x,false,unfiltered_in)?;
                    let lvalue_reg = addf!(self,RefSValue(name.to_string(),f.clone()),lvalue_subreg);
                    let rvalue_reg = addf!(self,SValue(name.to_string(),f.clone()),rvalue_subreg);
                    Ok((lvalue_reg,fvalue_reg,rvalue_reg))
                } else {
                    Err("Can only take \"dot\" of structs".to_string())
                }
            },
            Expression::Pling(x,f) => {
                if let ExpressionType::Base(BaseType::EnumType(name)) = self.type_of(x)? {
                    let (lvalue_subreg,fvalue_subreg,rvalue_subreg) = self.build_lvalue(x,false,unfiltered_in)?;
                    let lvalue_reg = addf!(self,RefEValue(name.to_string(),f.clone()),lvalue_subreg);
                    let mut fvalue_reg = addf!(self,FilterEValue(name.to_string(),f.clone()),rvalue_subreg);
                    if let Some(fvalue_subreg) = fvalue_subreg {
                        fvalue_reg = addf!(self,ReFilter,fvalue_subreg,fvalue_reg);
                    }
                    let rvalue_reg = addf!(self,EValue(name.to_string(),f.clone()),rvalue_subreg);
                    Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
                } else {
                    Err("Can only take \"pling\" of enums".to_string())
                }
            },
            Expression::Square(x) => {
                let (lvalue_subreg,_,rvalue_subreg) = self.build_lvalue(x,false,false)?;
                let lvalue_reg = addf!(self,RefSquare,lvalue_subreg);
                let rvalue_reg = addf!(self,Square,rvalue_subreg);
                let fvalue_reg = addf!(self,FilterSquare,rvalue_subreg);
                Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
            },
            Expression::Filter(x,f) => {
                let (lvalue_reg,fvalue_subreg,rvalue_subreg) = self.build_lvalue(x,false,false)?;
                /* Unlike in a bracket, @ makes no sense in a filter as the array has already been lost */
                let filterreg = self.build_rvalue(f,Some(&rvalue_subreg),None)?;
                let fvalue_reg = addf!(self,Filter,fvalue_subreg.unwrap(),filterreg);
                let rvalue_reg = addf!(self,Filter,rvalue_subreg,filterreg);
                Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
            },
            Expression::Bracket(x,f) => {
                let (lvalue_subreg,_,rvalue_subreg) = self.build_lvalue(x,false,false)?;
                let lvalue_reg = addf!(self,RefSquare,lvalue_subreg);
                let rvalue_interreg = addf!(self,Square,rvalue_subreg);
                let fvalue_interreg = addf!(self,FilterSquare,rvalue_subreg);
                let atreg = addf!(self,At,rvalue_subreg);
                let filterreg = self.build_rvalue(f,Some(&rvalue_interreg),Some(&atreg))?;
                let fvalue_reg = addf!(self,Filter,fvalue_interreg,filterreg);
                let rvalue_reg = addf!(self,Filter,rvalue_interreg,filterreg);
                Ok((lvalue_reg,Some(fvalue_reg),rvalue_reg))
            },
            _ => return Err("Invalid lvalue".to_string())
        }
    }

    fn build_rvalue(&mut self, expr: &Expression, dollar: Option<&Register>, at: Option<&Register>) -> Result<Register,String> {
        Ok(match expr {
            Expression::Identifier(id) => {
                if !self.regnames.contains_key(id) {
                    return Err(format!("Unset variable {:?}",id));
                }
                let real_reg = self.regnames[id];
                addf!(self,Copy,real_reg)
            },
            Expression::Number(n) =>        addf!(self,NumberConst(*n)),
            Expression::LiteralString(s) => addf!(self,StringConst(s.to_string())),
            Expression::LiteralBool(b) =>   addf!(self,BooleanConst(*b)),
            Expression::LiteralBytes(b) =>  addf!(self,BytesConst(b.to_vec())),
            Expression::Vector(v) =>        self.build_vec(v,dollar,at)?,
            Expression::Operator(name,x) => {
                let mut subregs = vec![];
                for e in x {
                    let r = self.build_rvalue(e,dollar,at)?;
                    subregs.push(r);
                }
                self.context.add_untyped_f(InstructionType::Operator(name.clone()),subregs)?
            },
            Expression::CtorStruct(s,x,n) => {
                let mut subregs = vec![];
                for e in x {
                    let r = self.build_rvalue(e,dollar,at)?;
                    subregs.push(r);
                }
                let out = self.struct_rearrange(s,subregs,n)?;
                self.context.add_untyped_f(InstructionType::CtorStruct(s.clone()),out)?
            },
            Expression::CtorEnum(e,b,x) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                addf!(self,CtorEnum(e.clone(),b.clone()),subreg)
            },
            Expression::Dot(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let stype = self.context.get_partial_type(&subreg);
                if let ExpressionType::Base(BaseType::StructType(name)) = stype {
                    addf!(self,SValue(name.to_string(),f.clone()),subreg)
                } else {
                    return Err(format!("Can only take \"dot\" of structs, not {:?}",stype));
                }
            },
            Expression::Query(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let etype = self.context.get_partial_type(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    addf!(self,ETest(name.to_string(),f.clone()),subreg)
                } else {
                    return Err("Can only take \"query\" of enums".to_string());
                }
            },
            Expression::Pling(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let etype = self.context.get_partial_type(&subreg);
                if let ExpressionType::Base(BaseType::EnumType(name)) = etype {
                    addf!(self,EValue(name.to_string(),f.clone()),subreg)
                } else {
                    return Err("Can only take \"pling\" of enums".to_string());
                }
            },
            Expression::Square(x) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                addf!(self,Square,subreg)
            },
            Expression::Star(x) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                addf!(self,Star,subreg)
            },
            Expression::Filter(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                /* Unlike in a bracket, @ makes no sense in a filter as the array has already been lost */
                let filterreg = self.build_rvalue(f,Some(&subreg),None)?;
                addf!(self,Filter,subreg,filterreg)
            },
            Expression::Bracket(x,f) => {
                let subreg = self.build_rvalue(x,dollar,at)?;
                let atreg = addf!(self,At,subreg);
                let sq_subreg = addf!(self,Square,subreg);
                let filterreg = self.build_rvalue(f,Some(&sq_subreg),Some(&atreg))?;
                addf!(self,Filter,sq_subreg,filterreg)
            },
            Expression::Dollar => {
                if let Some(dollar) = dollar {
                    addf!(self,Copy,*dollar)
                } else {
                    return Err("Unexpected $".to_string());
                }
            },
            Expression::At => {
                if let Some(at) = at {
                    addf!(self,Copy,*at)
                } else {
                    return Err("Unexpected @".to_string());
                }
            }
        })
    }

    fn build_stmt(&mut self, stmt: &Statement) -> Result<(),String> {
        let mut regs = Vec::new();
        let mut modes = Vec::new();
        let procdecl = self.defstore.get_proc(&stmt.0);
        if procdecl.is_none() {
            return Err(format!("No such procedure '{}'",stmt.0));
        }
        for (i,member) in procdecl.unwrap().get_signature().each_member().enumerate() {
            match member {
                SignatureMemberConstraint::RValue(_) => {
                    modes.push(MemberMode::RValue);
                    regs.push(self.build_rvalue(&stmt.1[i],None,None)?);
                },
                SignatureMemberConstraint::LValue(_) => {
                    let (lvalue_reg,fvalue_reg,_) = self.build_lvalue(&stmt.1[i],true,true)?;
                    if let Some(fvalue_reg) = fvalue_reg {
                        modes.push(MemberMode::FValue);
                        regs.push(fvalue_reg);
                    }
                    modes.push(MemberMode::LValue);
                    regs.push(lvalue_reg);
                }
            }
        }
        self.context.add_untyped(Instruction::new(InstructionType::Proc(stmt.0.to_string(),modes),regs))?;
        Ok(())
    }

    fn go(mut self, stmts: Vec<Statement>) -> Result<GenContext<'a>,Vec<String>> {
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
            self.context.generate_types();
            Ok(self.context)
        }
    }
}

pub fn generate_code<'a>(defstore: &'a DefStore, stmts: Vec<Statement>) -> Result<GenContext,Vec<String>> {
    let mut context = CodeGen::new(defstore).go(stmts)?;
    print!("c {:?}\n",context.get_instructions().len());
    context.phase_finished();
    print!("d {:?}\n",context.get_instructions().len());
    Ok(context)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::Parser;
    use crate::test::files::load_testdata;

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
        let gencontext = generate_code(&defstore,stmts).expect("codegen");
        let cmds : Vec<String> = gencontext.get_instructions().iter().map(|e| format!("{:?}",e)).collect();
        let outdata = load_testdata(&["codegen","generate-smoke2.out"]).ok().unwrap();
        print!("{}",cmds.join(""));
        assert_eq!(outdata,cmds.join(""));
    }

    #[test]
    fn codegen_lvalue_checks() {
        run_pass("typepass-reassignok.dp").expect("A");
        run_pass("typepass-reassignbad.dp").expect_err("B");
    }
}
