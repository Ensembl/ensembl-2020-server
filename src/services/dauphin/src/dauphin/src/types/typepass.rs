use std::fmt;

use super::argumentmatch::ArgumentMatch;
use crate::codegen::Instruction;
use super::typeinf::{ TypeInf, Referrer };
use super::typestep::type_step;
use crate::codegen::Register;
use crate::codegen::DefStore;
use super::uniquifier::Uniquifier;
use crate::lexer::{ FileResolver, Lexer };
use crate::parser::parse_signature;
use super::types::{ ArgumentType, TypeSig, TypeSigExpr, BaseType };

fn sig_gen(sig: &str,defstore: &DefStore) -> Result<ArgumentType,String> {
    let resolver = FileResolver::new();
    let mut lexer = Lexer::new(resolver);
    lexer.import(&format!("data: {}",sig)).ok();
    parse_signature(&mut lexer,defstore).map_err(|_| "internal sig parsing failed".to_string())
}

#[derive(Clone)]
pub struct TypePass {
    uniquifier: Uniquifier,
    typeinf: TypeInf,
    allow_typechange: bool
}

impl TypePass {
    pub fn new(allow_typechange: bool) -> TypePass {
        TypePass {
            uniquifier: Uniquifier::new(),
            typeinf: TypeInf::new(),
            allow_typechange
        }
    }

    pub fn get_typeinf(&mut self) -> &mut TypeInf { &mut self.typeinf }

    // TODO move Result into get methods
    fn extract_proc_sig_regs(name: &str, defstore: &DefStore, regs: &Vec<Register>) -> Result<Vec<(ArgumentType,Register)>,String> {
        let procdecl = defstore.get_proc(name).ok_or_else(|| format!("No such procedure {:?}",name))?;
        let sigs = procdecl.sigs();
        if regs.len() != sigs.len() {
            return Err(format!("Incorrect number of arguments to {}",name));
        }
        let mut out = Vec::new();
        for (i,s) in sigs.iter().enumerate() {
            out.push((s.clone(),regs[i].clone()));
        }
        Ok(out)
    }

    fn get_upstream(&mut self, reg: &Register) -> Result<Register,String> {
        let sig = self.typeinf.get_sig(&Referrer::Register(reg.clone())).clone();
        match sig {
            TypeSig::Left(_,reg) => Ok(reg),
            TypeSig::Right(_) => Err("Expected reference".to_string())
        }
    }

    pub fn extract_sig_regs(&mut self,instr: &Instruction, defstore: &DefStore) -> Result<Vec<ArgumentMatch>,String> {
        let out = match instr {
            Instruction::Proc(name,regs) => TypePass::extract_proc_sig_regs(name,defstore,regs),
            Instruction::NumberConst(reg,_) => Ok(vec![(sig_gen("writeonly number",defstore)?,reg.clone())]),
            Instruction::BooleanConst(reg,_) => Ok(vec![(sig_gen("writeonly boolean",defstore)?,reg.clone())]),
            Instruction::StringConst(reg,_) => Ok(vec![(sig_gen("writeonly string",defstore)?,reg.clone())]),
            Instruction::BytesConst(reg,_) => Ok(vec![(sig_gen("writeonly bytes",defstore)?,reg.clone())]),
            Instruction::List(reg) => Ok(vec![(sig_gen("writeonly vec(_)",defstore)?,reg.clone())]),
            Instruction::Star(dst,src) => Ok(vec![
                (sig_gen("writeonly vec(_A)",defstore)?,dst.clone()),
                (sig_gen("_A",defstore)?,src.clone())
            ]),
            Instruction::Square(dst,src) => Ok(vec![
                (sig_gen("writeonly _A",defstore)?,dst.clone()),
                (sig_gen("vec(_A)",defstore)?,src.clone())
            ]),
            Instruction::At(dst,src) => Ok(vec![
                (sig_gen("writeonly number",defstore)?,dst.clone()),
                (sig_gen("_",defstore)?,src.clone())
            ]),
            Instruction::Filter(dst,src,filter) => Ok(vec![
                (sig_gen("writeonly _A",defstore)?,dst.clone()),
                (sig_gen("_A",defstore)?,src.clone()),
                (sig_gen("boolean",defstore)?,filter.clone()),
            ]),
            Instruction::Push(dst,src) => Ok(vec![
                (sig_gen("writeonly vec(_A)",defstore)?,dst.clone()),
                (sig_gen("_A",defstore)?,src.clone()),
            ]),
            Instruction::CtorEnum(name,branch,dst,src) => {
                let exprdecl = defstore.get_enum(name).ok_or_else(|| format!("No such enum {:?}",name))?;
                let base = BaseType::EnumType(name.to_string());
                let branch_typedef = exprdecl.get_branch_type(branch)
                        .ok_or_else(|| format!("No such enum branch {:?}",name))?
                        .to_typesigexpr();
                Ok(vec![
                    (ArgumentType::new_right_writeonly(&TypeSigExpr::Base(base)),dst.clone()),
                    (ArgumentType::new_right(&branch_typedef),src.clone()),
                ])
            },
            Instruction::CtorStruct(name,dst,srcs) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_struct(name).ok_or_else(|| format!("No such struct {:?}",name))?;
                let base = BaseType::StructType(name.to_string());
                let dst_sig = ArgumentType::new_right_writeonly(&TypeSigExpr::Base(base));
                let intypes : Vec<TypeSigExpr> = exprdecl.get_member_types().iter()
                                .map(|x| x.to_typesigexpr()).collect();
                if srcs.len() != intypes.len() {
                    return Err("Incorrect number of arguments".to_string());
                }
                out.push((dst_sig,dst.clone()));
                for (i,intype) in intypes.iter().enumerate() {
                    out.push((ArgumentType::new_right(intype),srcs.get(i).unwrap().clone()));
                }
                Ok(out)
            },
            Instruction::SValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                let stype = TypeSigExpr::Base(BaseType::StructType(stype.to_string()));
                Ok(vec![(ArgumentType::new_right_writeonly(&dtype.to_typesigexpr()),dst.clone()),
                        (ArgumentType::new_right(&stype),src.clone())])
            },
            Instruction::EValue(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                let etype = TypeSigExpr::Base(BaseType::EnumType(etype.to_string()));
                Ok(vec![(ArgumentType::new_right_writeonly(&dtype.to_typesigexpr()),dst.clone()),
                        (ArgumentType::new_right(&etype),src.clone())])
            },
            Instruction::ETest(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                exprdecl.get_branch_type(field).ok_or_else(|| format!("No such branch {:?}",field))?;
                let etype = TypeSigExpr::Base(BaseType::EnumType(etype.to_string()));
                Ok(vec![(sig_gen("writeonly boolean",defstore)?,dst.clone()),
                        (ArgumentType::new_right(&etype),src.clone())])
            },
            Instruction::RefSValue(field,stype,dst,src) => {
                let exprdecl = defstore.get_struct(stype).ok_or_else(|| format!("No such struct {:?}",stype))?;
                let dtype = exprdecl.get_member_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                let stypesig = TypeSigExpr::Base(BaseType::StructType(stype.to_string()));
                let upstream = self.get_upstream(src)?;
                let newreg = Register::Left(Box::new(upstream.clone()),field.to_string());
                let newtype = TypeSig::Left(dtype.to_typesigexpr(),newreg.clone());
                self.typeinf.add(&Referrer::Register(newreg.clone()),&newtype);
                Ok(vec![
                    (ArgumentType::new_left_writeonly(&dtype.to_typesigexpr(),&newreg),dst.clone()),
                    (ArgumentType::new_left(&stypesig,&upstream),src.clone())
                ])
            },
            Instruction::RefEValue(field,etype,dst,src) => {
                let exprdecl = defstore.get_enum(etype).ok_or_else(|| format!("No such enum {:?}",etype))?;
                let dtype = exprdecl.get_branch_type(field).ok_or_else(|| format!("No such field {:?}",field))?;
                let stypesig = TypeSigExpr::Base(BaseType::EnumType(etype.to_string()));
                let upstream = self.get_upstream(src)?;
                let newreg = Register::Left(Box::new(upstream.clone()),field.to_string());
                let newtype = TypeSig::Left(dtype.to_typesigexpr(),newreg.clone());
                self.typeinf.add(&Referrer::Register(newreg.clone()),&newtype);
                Ok(vec![
                    (ArgumentType::new_left_writeonly(&dtype.to_typesigexpr(),&newreg),dst.clone()),
                    (ArgumentType::new_left(&stypesig,&upstream),src.clone())
                ])
            },
            Instruction::RefSquare(dst,src) => {
                let upstream = self.get_upstream(src)?;
                let newreg = Register::Left(Box::new(upstream.clone()),"+".to_string());
                let newtype = ArgumentType::new_left_writeonly(&sig_gen("_A",defstore)?.get_intype().expr(),&newreg);
                self.typeinf.add(&Referrer::Register(newreg.clone()),&newtype.get_intype());
                Ok(vec![
                    (ArgumentType::new_left_writeonly(&sig_gen("_A",defstore)?.get_intype().expr(),&newreg),dst.clone()),
                    (ArgumentType::new_left(&sig_gen("vec(_A)",defstore)?.get_intype().expr(),&upstream),src.clone())
                ])
            },
            Instruction::RefFilter(dst,src,filter) => {
                let upstream = self.get_upstream(src)?;
                let newreg = Register::Left(Box::new(upstream.clone()),"f".to_string());
                let newtype = ArgumentType::new_left_writeonly(&sig_gen("_A",defstore)?.get_intype().expr(),&newreg);
                self.typeinf.add(&Referrer::Register(newreg.clone()),&newtype.get_intype());
                Ok(vec![
                    (ArgumentType::new_left_writeonly(&sig_gen("_A",defstore)?.get_intype().expr(),&newreg),dst.clone()),
                    (ArgumentType::new_left(&sig_gen("_A",defstore)?.get_intype().expr(),&upstream),upstream.clone()),
                    (ArgumentType::new_right(&sig_gen("boolean",defstore)?.get_intype().expr()),filter.clone())
                ])
            },
            Instruction::Operator(name,dst,srcs) => {
                let mut out = Vec::new();
                let exprdecl = defstore.get_func(name).ok_or_else(|| format!("No such function {:?}",name))?;
                let dst_sig = ArgumentType::new_writeonly(&TypeSig::Right(exprdecl.get_dst().clone()));
                let intypes : &Vec<TypeSigExpr> = exprdecl.get_srcs();
                if srcs.len() != intypes.len() {
                    return Err("Incorrect number of arguments".to_string());
                }
                out.push((dst_sig,dst.clone()));
                for (i,intype) in intypes.iter().enumerate() {
                    out.push((ArgumentType::new(&TypeSig::Right(intype.clone())),srcs.get(i).unwrap().clone()));
                }
                Ok(out)
            },
            Instruction::Copy(dst,src) => Ok(vec![
                (sig_gen("writeonly _A",defstore)?,dst.clone()),
                (sig_gen("_A",defstore)?,src.clone())
            ]),
            Instruction::Ref(dst,src) => Ok(vec![
                (sig_gen("_ becomes _A",defstore)?,dst.clone()),
                (sig_gen("_A",defstore)?,src.clone())
            ])
        }?;
        Ok(out.iter().map(|(a,b)| ArgumentMatch::new(a,b)).collect())
    }

    // TODO ref invalidation
    /* ref is special as the root of all leftyness! */
    fn try_apply_ref(&mut self, dst: &Register, src: &Register) -> Result<(),String> {
        let dst_t = self.typeinf.new_register(dst);
        let sig = self.typeinf.get_sig(&Referrer::Register(src.clone())).clone();
        self.typeinf.add(&dst_t,&TypeSig::Left(sig.expr().clone(),src.clone()));
        Ok(())
    }

    /* everything that's not ref */
    fn try_apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let sig_regs = self.extract_sig_regs(instr,defstore)?;
        let typesig = self.uniquifier.uniquify_sig(&sig_regs);
        type_step(&mut self.typeinf,&typesig,self.allow_typechange)
    }

    pub fn apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        //print!("ac {:?}",instr);
        let x = match instr {
            Instruction::Ref(dst,src) => self.try_apply_ref(dst,src),
            instr => self.try_apply_command(instr,defstore)
        };
        match x {
            Ok(_) => self.typeinf.commit(),
            Err(_) => self.typeinf.rollback()
        };
        x
    }
}

impl fmt::Debug for TypePass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.typeinf)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser };
    use crate::codegen::{ Generator, RegisterAllocator };

    fn run_pass(filename: &str, allow_typechange: bool) -> Result<(),Vec<String>> {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("test:codegen/{}",filename)).expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let regalloc = RegisterAllocator::new();
        let gen = Generator::new(&regalloc);
        let instrs : Vec<Instruction> = gen.go(&defstore,stmts)?;
        let mut tp = TypePass::new(allow_typechange);
        for instr in &instrs {
            tp.apply_command(instr,&defstore).map_err(|e| vec![e])?;
        }
        Ok(())
    }

    #[test]
    fn typepeass_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/typepass-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let regalloc = RegisterAllocator::new();
        let gen = Generator::new(&regalloc);
        let instrs : Vec<Instruction> = gen.go(&defstore,stmts).expect("codegen");
        let instrs_str : Vec<String> = instrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("{}\n",instrs_str.join(""));
        let mut tp = TypePass::new(true);
        for instr in &instrs {
            print!("=== {:?}",instr);
            let before = tp.clone();
            tp.apply_command(instr,&defstore).expect("ok");
            print!("{}",tp.typeinf.make_diff(&before.typeinf));
        }
    }

    #[test]
    fn typepass_lvalue_checks() {
        run_pass("typepass-reassignok.dp",true).expect("A");
        run_pass("typepass-reassignok.dp",false).expect_err("B");
        run_pass("typepass-reassignbad.dp",true).expect_err("C");
        run_pass("typepass-reassignbad.dp",false).expect_err("D");
    }
}
