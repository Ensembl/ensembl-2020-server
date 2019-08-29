use std::collections::HashMap;
use super::argumentmatch::ArgumentMatch;
use super::signaturematch::SignatureMatch;
use super::types::{ ArgumentType, TypeSig, TypeSigExpr };

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

    fn unique_member_sig(&mut self, names: &mut HashMap<String,String>, sig: &ArgumentType) -> ArgumentType {
        let typesig = self.unique_member_typesig(names,&sig.get_intype());
        let lvalue = sig.lvalue.as_ref().map(|lvalue| self.unique_member_typesig(names,&lvalue));
        ArgumentType { lvalue, writeonly: sig.writeonly, typesig }
    }

    pub fn uniquify_sig(&mut self, sig: &SignatureMatch) -> SignatureMatch {
        let mut names = HashMap::new();
        SignatureMatch::new(&sig.each_argument().map(|m| {
            ArgumentMatch::new(&self.unique_member_sig(&mut names,m.get_type()),m.get_register())
        }).collect())
    }
}
