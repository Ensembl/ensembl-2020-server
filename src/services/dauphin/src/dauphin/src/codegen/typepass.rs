use std::collections::HashMap;

use super::instruction::Instruction;
use super::typeinf::{ TypeInf, Referrer };
use super::register::Register;
use super::definitionstore::DefStore;
use crate::parser::{ Sig, TypeSig, BaseType };

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

    fn unique_member_typesig(&mut self, names: &mut HashMap<String,String>, sig: &TypeSig) -> TypeSig {
        match sig {
            TypeSig::Placeholder(p) =>
                TypeSig::Placeholder(names.entry(p.to_string()).or_insert_with(|| self.new_placeholder()).to_string()),
            TypeSig::Vector(v) =>
                TypeSig::Vector(Box::new(self.unique_member_typesig(names,v))),
            TypeSig::Base(v) => TypeSig::Base(v.clone())
        }
    }

    fn unique_member_sig(&mut self, names: &mut HashMap<String,String>, sig: &Sig) -> Sig {
        Sig { lvalue: sig.lvalue, out: sig.out, reverse: sig.reverse, typesig: self.unique_member_typesig(names,&sig.typesig) }
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

    fn extract_sig_regs(&self, instr: &Instruction, defstore: &DefStore) -> Result<Vec<(Sig,Register)>,String> {
        match instr {
            Instruction::Proc(name,regs) => self.extract_proc_sig_regs(name,defstore,regs),
            Instruction::NumberConst(reg,_) => Ok(vec![(Sig { lvalue: true, out: true, reverse: false, typesig: TypeSig::Base(BaseType::NumberType) },reg.clone())]),
            Instruction::BooleanConst(reg,_) => Ok(vec![(Sig { lvalue: true, out: true, reverse: false, typesig: TypeSig::Base(BaseType::BooleanType) },reg.clone())]),
            Instruction::Ref(dst,src) => {
                Ok(vec![(Sig { lvalue: true, out: true, reverse: false, typesig: TypeSig::Placeholder("A".to_string()) },dst.clone()),
                        (Sig { lvalue: false, out: false, reverse: true, typesig: TypeSig::Placeholder("A".to_string()) },src.clone())])
            },
            _ => Err(format!("no signature for {:?}",instr))
        }
    }

    pub fn try_apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let (sig_regs) = self.extract_sig_regs(instr,defstore)?;
        let typesig = self.uniqueize(&sig_regs);
        let mut unifies = Vec::new();
        let mut check_valid = Vec::new();
        for (sig,reg) in &typesig {
            let reg = self.typeinf.new_register(reg);
            if sig.out {
                self.typeinf.remove(&reg);
                let ph = self.new_placeholder().clone();
                self.typeinf.add(&reg,&TypeSig::Placeholder(ph));
            } else {
                check_valid.push(reg.clone());
            }
            let tmp = self.typeinf.new_temp().clone();
            self.typeinf.add(&tmp,&sig.typesig);

            unifies.push((reg,tmp));
            
        }
        for (reg,tmp) in &unifies {
            self.typeinf.unify(&reg,&tmp)?;
        }
        for reg in &check_valid {
            let sig = self.typeinf.get_sig(reg);
            if sig.is_invalid() {
                //return Err(format!("Use of invalid value from {:?}",reg));
            }
        }
        Ok(())
    }

    pub fn apply_command(&mut self, instr: &Instruction, defstore: &DefStore) -> Result<(),String> {
        let x = self.try_apply_command(instr,defstore);
        match x {
            Ok(_) => self.typeinf.commit(),
            Err(_) => self.typeinf.rollback()
        };
        x
    }
}

// TODO proper register names

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
            tp.apply_command(instr,&defstore).expect("ok");
            print!("{:?}\n",tp.typeinf);
        }
    }
}
