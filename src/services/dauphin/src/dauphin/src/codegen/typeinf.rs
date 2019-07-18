use std::collections::{ HashMap, HashSet };
use crate::parser::TypeSig;
use super::register::Register;

#[derive(Debug,Clone,Hash,PartialEq,Eq)]
enum Referrer {
    Register(Register),
    Temporary(u32)
}

struct TypeInfStore {
    signatures: HashMap<Referrer,TypeSig>,
    uses_placeholder: HashMap<String,HashSet<Referrer>>
}

impl TypeInfStore {
    fn new() -> TypeInfStore {
        TypeInfStore {
            signatures: HashMap::new(),
            uses_placeholder: HashMap::new()
        }
    }

    fn remove(&mut self, reg: &Referrer) {
        if let Some(typesig) = self.signatures.remove(reg) {
            if let Some(ph) = typesig.get_placeholder() {
                self.uses_placeholder.entry(ph.to_string()).or_insert_with(|| HashSet::new()).remove(reg);
            }
        }
        self.signatures.remove(reg);
    }

    fn get_sig(&self, reg: &Referrer) -> Option<&TypeSig> {
        self.signatures.get(reg)
    }

    fn add(&mut self, reg: &Referrer, typesig: &TypeSig) {
        if self.signatures.contains_key(reg) {
            self.remove(reg);
        }
        if let Some(ph) = typesig.get_placeholder() {
            self.uses_placeholder.entry(ph.to_string()).or_insert_with(|| HashSet::new()).insert(reg.clone());
        }
        self.signatures.insert(reg.clone(),typesig.clone());
    }

    fn all_using(&self, ph: &str) -> HashSet<Referrer> {
        if let Some(ref reg_set) = self.uses_placeholder.get(ph).cloned() {
            reg_set.iter().cloned().collect()
        } else {
            HashSet::new()
        }
    }
}

struct TypeInf {
    next_temp: u32,
    store: TypeInfStore
}

impl TypeInf {
    pub fn new() -> TypeInf {
        TypeInf {
            next_temp: 0,
            store: TypeInfStore::new()
        }
    }

    pub fn remove(&mut self, reg: &Referrer) {
        self.store.remove(reg);
    }

    pub fn get_sig(&self, reg: &Referrer) -> Option<&TypeSig> {
        self.store.get_sig(reg)
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

    fn add_equiv(&mut self, ph: &str, val: &TypeSig) {
        for reg in &self.store.all_using(ph) {
            if let Some(old_val) = self.store.get_sig(reg) {
                let new_val = TypeInf::updated_sig(old_val,val);
                self.add(reg,&new_val);
            }
        }
    }

    fn extract_equiv(&mut self, a: &TypeSig, b: &TypeSig) -> Result<Option<(String,TypeSig)>,()> {
        let out = match (a,b) {
            (TypeSig::Base(a_v),TypeSig::Base(b_v)) => {
                if a_v == b_v { Ok(None) } else { Err(()) }
            },
            (TypeSig::Vector(a_v),TypeSig::Vector(b_v)) => self.extract_equiv(a_v,b_v),
            (TypeSig::Placeholder(a_v),b) => {
                Ok(Some((a_v.to_string(),b.clone())))
            },
            (a,TypeSig::Placeholder(b_v)) => {
                Ok(Some((b_v.to_string(),a.clone())))
            },
            _ => Err(())
        }?;
        if let Some((ref ph,ref new_val)) = out {
            if &TypeSig::Placeholder(ph.to_string()) == new_val {
                print!("rec\n");
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

    fn updated_sig(old_val: &TypeSig, repl: &TypeSig) -> TypeSig {
        match old_val {
            TypeSig::Base(v) => TypeSig::Base(v.clone()),
            TypeSig::Vector(v) => TypeSig::Vector(Box::new(TypeInf::updated_sig(v,repl))),
            TypeSig::Placeholder(_) => repl.clone()
        }
    }

    pub fn unify(&mut self, a_reg: &Referrer, b_reg: &Referrer) -> Result<(),String> {
        let a_sig = self.get_sig(a_reg).ok_or_else(|| format!("No type for {:?}",a_reg))?.clone();
        let b_sig = self.get_sig(b_reg).ok_or_else(|| format!("No type for {:?}",b_reg))?.clone();
        if let Some((ph,val)) = self.extract_equiv(&a_sig,&b_sig).map_err(|_|
            format!("Cannot unify types {:?} and {:?}",a_sig,b_sig)
        )? {
            self.add_equiv(&ph,&val);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lexer::{ FileResolver, Lexer };
    use crate::parser::{ Parser, parse_typesig };
    use crate::testsuite::load_testdata;

    fn typesig_gen(sig: &str) -> TypeSig {
        let resolver = FileResolver::new();
        let mut lexer = Lexer::new(resolver);
        lexer.import(&format!("data: {}",sig)).ok();
        parse_typesig(&mut lexer).expect("bad typesig")
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
        let a = ti.new_temp();
        let b = ti.new_temp();
        let c = ti.new_temp();
        ti.add(&a,&typesig_gen("vec(_A)"));
        ti.add(&b,&typesig_gen("vec(vec(string))"));
        ti.add(&c,&typesig_gen("_A"));
        ti.unify(&a,&b).expect("smoke");
        print!("{:?}\n",ti.get_sig(&a));
        print!("{:?}\n",ti.get_sig(&b));
        print!("{:?}\n",ti.get_sig(&c));
    }
}
