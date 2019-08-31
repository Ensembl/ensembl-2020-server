use std::fmt;

use std::collections::{ HashMap, HashSet };

use crate::codegen::Register2;
use super::types::{ ArgumentConstraint, RegisterType, InstructionConstraint, ExpressionType, BaseType };
use super::typesinternal::{ Key, TypeConstraint };
use super::typestore::TypeStore;

pub struct Typing {
    next: usize,
    store: TypeStore,
    regmap: HashMap<Register2,usize>,
    reg_isref: HashSet<Register2>
}

impl fmt::Debug for Typing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map : Vec<(Register2,usize)> = self.regmap.iter().map(|(k,v)| (k.clone(),v.clone())).collect();
        map.sort();
        for (reg,reg_id) in &map {
            write!(f,"{:?} = ",reg)?;
            if self.reg_isref.contains(reg) { write!(f,"ref(")?; }
            let type_ = self.store.get(&Key::External(*reg_id)).unwrap();
            write!(f,"{:?}",type_)?;
            if self.reg_isref.contains(reg) { write!(f,")")?; }
            write!(f,"\n")?;
        }
        Ok(())
    }
}


impl Typing {
    pub fn new() -> Typing {
        Typing {
            next: 0,
            store: TypeStore::new(),
            regmap: HashMap::new(),
            reg_isref: HashSet::new()
        }
    }

    fn extract(&mut self, in_: &InstructionConstraint) -> Vec<(TypeConstraint,Register2)> {
        let mut out = Vec::new();
        let mut name = HashMap::new();
        for (argument_constraint,register) in in_.each_member() {
            let type_constraint =
                TypeConstraint::from_argumentconstraint(&argument_constraint,|s| {
                    let next_val = self.next;
                    let val = *name.entry(s.to_string()).or_insert(next_val);
                    if val == next_val { self.next += 1; }
                    val
                });
            out.push((type_constraint,register.clone()));
        }
        out
    }

    pub fn add(&mut self, sig: &InstructionConstraint) -> Result<(),String> {
        for (constraint,register) in self.extract(sig) {
            let is_ref = match constraint {
                TypeConstraint::Reference(_) => true,
                TypeConstraint::NonReference(_) => false
            };
            if self.regmap.contains_key(&register) {
                if self.reg_isref.contains(&register) != is_ref {
                    return Err(format!("Cannot unify reference and non-reference"));
                }
            }
            if is_ref {
                self.reg_isref.insert(register.clone());
            }
            let next_val = self.next;
            let reg_id = *self.regmap.entry(register.clone()).or_insert(next_val);
            if reg_id == next_val { self.next += 1; }
            self.store.add(&Key::External(reg_id),constraint.get_expressionconstraint())?;
        }
        Ok(())
    }

    pub fn get(&self, reg: &Register2) -> ExpressionType {
        if let Some(reg_id) = self.regmap.get(reg) {
            if let Some(out) = self.store.get(&Key::External(*reg_id)) {
                return out;
            }
        }
        ExpressionType::Base(BaseType::Invalid)
    }
}