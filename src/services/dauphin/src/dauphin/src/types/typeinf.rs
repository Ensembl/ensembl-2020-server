use std::collections::{ HashMap, HashSet };
use std::fmt;
use crate::parser::{ TypeSig, BaseType, TypeSigExpr };
use crate::codegen::Register;

#[derive(Clone,Hash,PartialEq,Eq,PartialOrd,Ord)]
pub enum Referrer {
    Register(Register),
    Temporary(u32)
}

impl Referrer {
    fn is_perm(&self) -> bool {
        match self {
            Referrer::Register(_) => true,
            Referrer::Temporary(_) => false
        }
    }
}

impl fmt::Debug for Referrer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Referrer::Register(n) => write!(f,"{:?}",n)?,
            Referrer::Temporary(i) => write!(f,"tmp({})",i)?
        }
        Ok(())
    }
}

#[derive(Clone)]
struct TypeInfStore {
    signatures: HashMap<Referrer,TypeSig>,
    signatures_txn: HashMap<Referrer,TypeSig>,
    signatures_txn_rm: HashSet<Referrer>,
    uses_placeholder: HashMap<String,HashSet<Referrer>>,
    uses_placeholder_txn: HashMap<String,HashSet<Referrer>>
}

impl TypeInfStore {
    fn new() -> TypeInfStore {
        TypeInfStore {
            signatures: HashMap::new(),
            signatures_txn: HashMap::new(),
            signatures_txn_rm: HashSet::new(),
            uses_placeholder: HashMap::new(),
            uses_placeholder_txn: HashMap::new()
        }
    }

    fn compile_ordered_list(&self) -> Vec<(&Referrer,&TypeSig)> {
        let mut out = Vec::new();
        for (reg,sig) in &self.signatures {
            if !self.signatures_txn.contains_key(reg) {
                out.push((reg,sig));
            }
        }
        for (reg,sig) in &self.signatures_txn {
            if !self.signatures_txn_rm.contains(reg) {
                out.push((reg,sig));
            }
        }
        out.sort();
        out
    }

    fn format(&self, e: &Vec<(&Referrer,&TypeSig)>) -> String {
        let mut out = String::new();
        for (reg,sig) in e {
            out.push_str(&format!("{:?} = {:?}\n",reg,sig));
        }
        out
    }

    fn make_diff(&self, other: &TypeInfStore) -> String {
        let self_list: HashSet<(&Referrer,&TypeSig)> =
            self.compile_ordered_list().drain(..).collect();
        let other_list: HashSet<(&Referrer,&TypeSig)> =
            other.compile_ordered_list().drain(..).collect();
        let changes = self_list.difference(&other_list).cloned().collect();
        self.format(&changes)
    }

    fn update_set(&mut self, ph: &str) -> &mut HashSet<Referrer> {
        if self.uses_placeholder_txn.contains_key(ph) {
            return self.uses_placeholder_txn.get_mut(ph).unwrap();
        } else {
            self.uses_placeholder.entry(ph.to_string()).or_insert_with(|| {
                HashSet::new()
            });
            self.uses_placeholder_txn.insert(ph.to_string(),self.uses_placeholder[ph].iter().cloned().collect());
            self.uses_placeholder_txn.get_mut(ph).unwrap()
        }
    }

    fn remove(&mut self, reg: &Referrer) {
        if let Some(typesig) = self.signatures.get(reg).cloned() {
            if let Some(ph) = typesig.get_placeholder() {
                self.update_set(ph).remove(reg);
            }
        }
        self.signatures_txn_rm.insert(reg.clone());
        self.signatures_txn.remove(reg);
    }

    fn get_sig(&self, reg: &Referrer) -> Option<&TypeSig> {
        if self.signatures_txn.contains_key(reg) {
            return self.signatures_txn.get(reg);
        }
        if self.signatures_txn_rm.contains(reg) {
            return None
        }
        self.signatures.get(reg)
    }

    fn add(&mut self, reg: &Referrer, typesig: &TypeSig) {
        if self.get_sig(reg).is_some() {
            self.remove(reg);
        }
        if let Some(ph) = typesig.get_placeholder() {
            self.update_set(ph).insert(reg.clone());
        }
        self.signatures_txn.insert(reg.clone(),typesig.clone());
        self.signatures_txn_rm.remove(reg);
    }

    fn all_using(&self, ph: &str) -> HashSet<Referrer> {
        if let Some(ref reg_set) = self.uses_placeholder_txn.get(ph).cloned() {
            reg_set.iter().cloned().collect()
        } else if let Some(ref reg_set) = self.uses_placeholder.get(ph).cloned() {
            reg_set.iter().cloned().collect()
        } else {
            HashSet::new()
        }
    }

    fn commit(&mut self) {
        for (ph,rr) in self.uses_placeholder_txn.drain() {
            let rr : HashSet<Referrer> = rr.iter().filter(|x| x.is_perm()).cloned().collect();
            self.uses_placeholder.insert(ph,rr);
        }
        for reg in self.signatures_txn_rm.drain() {
            self.signatures.remove(&reg);
        }
        for (reg,sig) in self.signatures_txn.drain() {
            if reg.is_perm() {
                self.signatures.insert(reg,sig);
            }
        }       
    }

    fn rollback(&mut self) {
        self.uses_placeholder_txn.clear();
        self.signatures_txn.clear();
        self.signatures_txn_rm.clear();
    }
}

impl fmt::Debug for TypeInfStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.format(&self.compile_ordered_list()))
    }
}

#[derive(Clone)]
pub struct TypeInf {
    next_temp: u32,
    store: TypeInfStore,
    invalid: TypeSig
}

impl TypeInf {
    pub fn new() -> TypeInf {
        TypeInf {
            next_temp: 0,
            store: TypeInfStore::new(),
            invalid: TypeSig::Right(TypeSigExpr::Base(BaseType::Invalid))
        }
    }

    pub fn get_sig(&mut self, reg: &Referrer) -> &TypeSig {
        if self.store.get_sig(reg).is_none() {
            self.store.add(reg,&self.invalid);
        }
        self.store.get_sig(reg).unwrap()
    }

    pub fn new_register(&mut self, reg: &Register) -> Referrer {
        Referrer::Register(reg.clone())
    }

    pub fn new_temp(&mut self) -> Referrer {
        self.next_temp += 1;
        Referrer::Temporary(self.next_temp)
    }

    pub fn add(&mut self, reg: &Referrer, typesig: &TypeSig) {
        self.store.add(reg,typesig);
    }

    // TODO distinct invalids
    fn add_equiv(&mut self, ph: &str, val: &TypeSigExpr) {
        for reg in &self.store.all_using(ph) {
            if let Some(old_val) = self.store.get_sig(reg) {
                let new_val = TypeInf::updated_sig(old_val,val);
                self.add(reg,&new_val);
            }
        }
    }

    fn extract_equivexpr(&mut self, a: &TypeSigExpr, b: &TypeSigExpr) -> Result<Option<(String,TypeSigExpr)>,()> {
        let out = match (a,b) {
            (TypeSigExpr::Base(a_v),TypeSigExpr::Base(b_v)) => {
                if a_v == b_v { Ok(None) } else { Err(()) }
            },
            (TypeSigExpr::Vector(a_v),TypeSigExpr::Vector(b_v)) =>
                self.extract_equivexpr(a_v,b_v),
            (TypeSigExpr::Placeholder(a_v),b) => {
                Ok(Some((a_v.to_string(),b.clone())))
            },
            (a,TypeSigExpr::Placeholder(b_v)) => {
                Ok(Some((b_v.to_string(),a.clone())))
            },
            _ => Err(())
        }?;
        if let Some((ref ph,ref new_val)) = out {
            if &TypeSigExpr::Placeholder(ph.to_string()) == new_val {
                return Ok(None);
            }
            if let Some(new_ph) = new_val.get_placeholder() {
                if new_ph == ph {
                    return Err(());
                }
            }
        }
        Ok(out)
    }

    fn extract_equiv(&mut self, a: &TypeSig, b: &TypeSig) -> Result<Option<(String,TypeSigExpr)>,()> {
        self.extract_equivexpr(a.expr(),b.expr())
    }

    fn updated_sigexpr(old_val: &TypeSigExpr, repl: &TypeSigExpr) -> TypeSigExpr {
        match old_val {
            TypeSigExpr::Base(v) => TypeSigExpr::Base(v.clone()),
            TypeSigExpr::Vector(v) => TypeSigExpr::Vector(Box::new(TypeInf::updated_sigexpr(v,repl))),
            TypeSigExpr::Placeholder(_) => repl.clone()
        }
    }

    fn updated_sig(old_val: &TypeSig, repl: &TypeSigExpr) -> TypeSig {
        match old_val {
            TypeSig::Left(old_val,reg) => 
                TypeSig::Left(TypeInf::updated_sigexpr(old_val,repl),reg.clone()),
            TypeSig::Right(old_val) => 
                TypeSig::Right(TypeInf::updated_sigexpr(old_val,repl))
        }
    }

    pub fn unify(&mut self, a_reg: &Referrer, b_reg: &Referrer) -> Result<(),String> {
        let a_sig = self.get_sig(a_reg).clone();
        let b_sig = self.get_sig(b_reg).clone();
        //print!("unify {:?} <-> {:?} ie {:?} <-> {:?}\n",a_reg,b_reg,a_sig,b_sig);
        if let Some((ph,val)) = self.extract_equiv(&a_sig,&b_sig).map_err(|_|
            format!("Cannot unify types {:?} and {:?}",a_sig,b_sig)
        )? {
            self.add_equiv(&ph,&val);
        } else if a_sig != b_sig {
            print!("static check\n");
            return Err(format!("Cannot unify {:?} and {:?}",a_sig,b_sig));
        }
        Ok(())
    }

    pub fn commit(&mut self) {
        self.store.commit();
    }

    pub fn rollback(&mut self) {
        self.store.rollback();
    }

    pub fn make_diff(&self, other: &TypeInf) -> String {
        self.store.make_diff(&other.store)
    }
}

impl fmt::Debug for TypeInf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{:?}",self.store)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ parse_typesig };

    fn typesig_gen(sig: &str) -> TypeSig {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("data: {}",sig)).ok();
        parse_typesig(&mut lexer).expect("bad typesig")
    }

    fn render(ts: &TypeSig) -> String {
        format!("{:?}",ts)
    }

    #[test]
    fn failed_unify() {
        let mut ti = TypeInf::new();
        let a = ti.new_temp();
        let b = ti.new_temp();
        ti.add(&a,&typesig_gen("string"));
        ti.add(&b,&typesig_gen("number"));
        ti.unify(&a,&b).expect_err("failed_unify");
    }

    #[test]
    fn recursive() {
        let mut ti = TypeInf::new();
        let a = ti.new_temp();
        let b = ti.new_temp();
        ti.add(&a,&typesig_gen("vec(_A)"));
        ti.add(&b,&typesig_gen("_A"));
        ti.unify(&a,&b).expect_err("recursive");
    }

    #[test]
    fn identity() {
        let mut ti = TypeInf::new();
        let a = ti.new_temp();
        let b = ti.new_temp();
        ti.add(&a,&typesig_gen("_A"));
        ti.add(&b,&typesig_gen("_A"));
        ti.unify(&a,&b).expect("identity");
    }

    #[test]
    fn typeinf_smoke() {
        let mut ti = TypeInf::new();
        let a = ti.new_register(&Register::Temporary(1));
        let b = ti.new_register(&Register::Temporary(2));
        let c = ti.new_register(&Register::Temporary(3));
        ti.add(&a,&typesig_gen("vec(_A)"));
        ti.add(&b,&typesig_gen("vec(vec(string))"));
        ti.add(&c,&typesig_gen("_A"));
        ti.unify(&a,&b).expect("smoke");
        assert_eq!("vec(vec(string))",render(ti.get_sig(&a)));
        assert_eq!("vec(vec(string))",render(ti.get_sig(&b)));
        assert_eq!("vec(string)",render(ti.get_sig(&c)));
        ti.commit();
        assert_eq!("vec(vec(string))",render(ti.get_sig(&a)));
        assert_eq!("vec(vec(string))",render(ti.get_sig(&b)));
        assert_eq!("vec(string)",render(ti.get_sig(&c)));
    }

    #[test]
    fn rollback() {
        let mut ti = TypeInf::new();
        let a = ti.new_register(&Register::Temporary(1));
        let b = ti.new_register(&Register::Temporary(2));
        let c = ti.new_register(&Register::Temporary(3));
        ti.add(&a,&typesig_gen("vec(_A)"));
        ti.add(&b,&typesig_gen("vec(vec(string))"));
        ti.add(&c,&typesig_gen("_A"));
        ti.commit();
        ti.unify(&a,&b).expect("smoke");
        assert_eq!("vec(vec(string))",render(ti.get_sig(&a)));
        assert_eq!("vec(vec(string))",render(ti.get_sig(&b)));
        assert_eq!("vec(string)",render(ti.get_sig(&c)));
        ti.rollback();
        assert_eq!("vec(_A)",render(ti.get_sig(&a)));
        assert_eq!("vec(vec(string))",render(ti.get_sig(&b)));
        assert_eq!("_A",render(ti.get_sig(&c)));
    }
}
