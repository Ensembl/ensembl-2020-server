use std::collections::HashMap;
use super::types::{ Sig, TypeSig, TypeSigExpr };
use crate::codegen::Register;

#[derive(Clone)]
pub struct Uniquifier {
    next_placeholder: u32
}

impl Uniquifier {
    pub fn new() -> Uniquifier {
        Uniquifier { next_placeholder: 0 }
    }

    pub fn new_placeholder(&mut self) -> String {
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
        let lvalue = sig.lvalue.as_ref().map(|lvalue| self.unique_member_typesig(names,&lvalue));
        Sig { lvalue, out: sig.out, typesig }
    }

    pub fn uniquify_sig(&mut self, sig: &Vec<(Sig,Register)>) -> Vec<(Sig,Register)> {
        let mut names = HashMap::new();
        sig.iter().map(|(s,r)| {
            (self.unique_member_sig(&mut names,&s),r.clone())
        }).collect()
    }
}
