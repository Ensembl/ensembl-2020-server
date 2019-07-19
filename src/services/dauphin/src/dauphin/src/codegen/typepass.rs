use std::collections::HashMap;

use super::instruction::Instruction;
use super::typeinf::{ TypeInf, Referrer };
use super::register::Register;
use super::definitionstore::DefStore;
use crate::lexer::{ FileResolver, Lexer };
use crate::parser::{ Sig, TypeSig, BaseType, TypeSigExpr, parse_signature };

fn sig_gen(sig: &str) -> Result<Sig,String> {
    let resolver = FileResolver::new();
    let mut lexer = Lexer::new(resolver);
    lexer.import(&format!("data: {}",sig)).ok();
    parse_signature(&mut lexer).map_err(|e| "internal sig parsing failed".to_string())
}

struct TypePass {
    next_placeholder: u32,
    typeinf: TypeInf
}

impl TypePass {
    pub fn new() -> TypePass {
        TypePass {
            next_placeholder: 0,
            typeinf: TypeInf::new()
        }
    }

    pub fn get_typeinf(&self) -> &TypeInf { &self.typeinf }

    fn new_placeholder(&mut self) -> String {
        self.next_placeholder += 1;
        self.next_placeholder.to_string()
    }

    fn get_name(&mut self, names: &mut HashMap<String,String>, p: &str) -> String {
        if p == "_" {
            self.new_placeholder()
        } else {
            names.entry(p.to_string()).or_insert_with(|| self.new_placeholder()).to_string()
        }
    }

    fn unique_member_typesigexpr(&mut self, names: &mut HashMap<String,String>, sig: &TypeSigExpr) -> TypeSigExpr {
        match sig {
            TypeSigExpr::Placeholder(p) =>
                TypeSigExpr::Placeholder(self.get_name(names,p)),
            TypeSigExpr::Vector(v) =>
                TypeSigExpr::Vector(Box::new(self.unique_member_typesigexpr(names,v))),
            TypeSigExpr::Base(v) => TypeSigExpr::Base(v.clone())
        }
    }

    fn unique_member_typesig(&mut self, names: &mut HashMap<String,String>, sig: &TypeSig) -> TypeSig {
        match sig {
            TypeSig::Left(x,reg) => TypeSig::Left(self.unique_member_typesigexpr(names,x),reg.clone()),
            TypeSig::Right(x) => TypeSig::Right(self.unique_member_typesigexpr(names,x)),
        }
    }

    fn unique_member_sig(&mut self, names: &mut HashMap<String,String>, sig: &Sig) -> Sig {
        let typesig = self.unique_member_typesig(names,&sig.typesig);
        let lvalue = sig.lvalue.as_ref().map(|lvalue| self.unique_member_typesigexpr(names,&lvalue));
        Sig { lvalue, out: sig.out, typesig }
    }

    fn uniqueize(&mut self, sig: &Vec<(Sig,Register)>) -> Vec<(Sig,Register)> {
        let mut names = HashMap::new();
        sig.iter().map(|(s,r)| {
            (self.unique_member_sig(&mut names,&s),r.clone())
        }).collect()
    }

    // TODO move Result into get methods
    fn extract_proc_sig_regs(&self, name: &str, defstore: &DefStore, regs: &Vec<Register>) -> Result<Vec<(Sig,Register)>,String> {
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

    // TODO as strings
    fn extract_sig_regs(&self, instr: &Instruction, defstore: &DefStore) -> Result<Vec<(Sig,Register)>,String> {
        match instr {
            Instruction::Proc(name,regs) => self.extract_proc_sig_regs(name,defstore,regs),
            Instruction::NumberConst(reg,_) => Ok(vec![(sig_gen("out number")?,reg.clone())]),
            Instruction::BooleanConst(reg,_) => Ok(vec![(sig_gen("out boolean")?,reg.clone())]),
            Instruction::StringConst(reg,_) => Ok(vec![(sig_gen("out string")?,reg.clone())]),
            Instruction::BytesConst(reg,_) => Ok(vec![(sig_gen("out bytes")?,reg.clone())]),
            Instruction::List(reg) => Ok(vec![(sig_gen("out vec(_)")?,reg.clone())]),
            Instruction::Push(dst,src) => Ok(vec![
                (sig_gen("out vec(_A)")?,dst.clone()),
                (sig_gen("_A")?,src.clone()),
            ]),
            _ => Err(format!("no signature for {:?}",instr))
        }
    }

    // TODO remove reverse
    /* ref is special as the root of all leftyness! */
    pub fn try_apply_ref(&mut self, dst: &Register, src: &Register, defstore: &DefStore) -> Result<(),String> {
        let dst_t = self.typeinf.new_register(dst);
        let dst_ph = TypeSigExpr::Placeholder(self.new_placeholder().clone());
        self.typeinf.add(&dst_t,&TypeSig::Left(dst_ph,src.clone()));
        Ok(())
    }

    pub fn try_apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let sig_regs = self.extract_sig_regs(instr,defstore)?;
        let typesig = self.uniqueize(&sig_regs);
        let mut unifies = Vec::new();
        let mut check_valid = Vec::new();
        let mut xform = Vec::new();
        for (sig,reg) in &typesig {
            let reg = self.typeinf.new_register(reg);
            if !sig.out {
                check_valid.push(reg.clone());
            }
            let tmp = self.typeinf.new_temp().clone();
            if sig.lvalue.is_some() {
                let ltmp = self.typeinf.new_temp();
                self.typeinf.add(&ltmp,&TypeSig::Right(sig.lvalue.as_ref().unwrap().clone()));
                xform.push((reg.clone(),ltmp,tmp.clone()));
            }
            self.typeinf.add(&tmp,&sig.typesig);
            unifies.push((reg,tmp));
        }
        for (reg,tmp) in &unifies {
            self.typeinf.unify(&reg,&tmp)?;
        }
        for reg in &check_valid {
            let sig = self.typeinf.get_sig(reg);
            if sig.is_invalid() {
                return Err(format!("Use of invalid value from {:?}",reg));
            }
        }
        for (reg,tmp,rtmp) in &xform {
            let tmp_sig = self.typeinf.get_sig(tmp).clone();
            let reg_sig = self.typeinf.get_sig(reg).clone();
            match &reg_sig {
                TypeSig::Left(x,r) => {
                    self.typeinf.unify(&Referrer::Register(r.clone()),rtmp)?;
                    self.typeinf.add(&Referrer::Register(r.clone()),&tmp_sig.clone());
                    self.typeinf.add(&reg,&TypeSig::Left(tmp_sig.expr().clone(),r.clone()));
                },
                TypeSig::Right(x) => {
                    self.typeinf.add(&reg,&tmp_sig.clone());
                }
            }
        }
        Ok(())
    }

    pub fn apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let x = match instr {
            Instruction::Ref(dst,src) => self.try_apply_ref(dst,src,defstore),
            instr => self.try_apply_command(instr,defstore)
        };
        match x {
            Ok(_) => self.typeinf.commit(),
            Err(_) => self.typeinf.rollback()
        };
        x
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser, parse_typesig };
    use crate::testsuite::load_testdata;
    use super::super::generate::Generator;

    #[test]
    fn typepeass_smoke() {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import("test:codegen/typepass-smoke.dp").expect("cannot load file");
        let p = Parser::new(lexer);
        let (stmts,defstore) = p.parse().expect("error");
        let gen = Generator::new();
        let instrs : Vec<Instruction> = gen.go(&defstore,stmts).expect("codegen");
        let instrs_str : Vec<String> = instrs.iter().map(|v| format!("{:?}",v)).collect();
        print!("{}\n",instrs_str.join(""));
        let mut tp = TypePass::new();
        for instr in &instrs {
            print!("=== {:?}",instr);
            tp.apply_command(instr,&defstore).expect("ok");
            print!("finish {:?}\n",tp.typeinf);
        }
    }
}