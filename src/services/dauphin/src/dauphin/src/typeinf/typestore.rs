use std::collections::{ HashMap, HashSet };

use super::types::RegisterType;
use super::typesinternal::{ ExpressionConstraint, Key, TypeConstraint };

/* Invariant: a Key is never both a key in self.values and at the same time used as a placeholder in
 * a value stored in self.values. As this invariant holds, we guarantee ourselves to be non-recursive.
 * 
 * During calls to self.add() to maintain the invariant we need to ensure both that we don't add a key
 * for a placeholder already in use, and also that we don't add a placeholder for a key already in use.
 * 
 * When adding, we first guard against the new placeholder matching an existing key. We check for an
 * existing key and substitute for its value. As the invariant hel before, the new placeholder is not a
 * key.
 * 
 * If the new key is present, then we plan to unify so will not add a new key, and so are safe if unification
 * is safe. Unification at most takes some value fragment (potentially including a placeholder in-use) and
 * uses it to subststute some other value (ie using it elsewhere). So it does not break the invariant.
 * 
 * If the new key is /not/ already present, first we must guard against adding an identical key and placeholder 
 * at the same time. A local recursion check ensures this. Checks to this point have not modified the store so
 * we can safely bomb-out if needed.
 * 
 * Next, as we plan to add a key, we need to guard against the new key being used as a placeholder. To avoid 
 * that, we run replace_placeholder on our key with its new value. We know the new placeholder doesn't match 
 * an existing key (previous step) so the invariant still holds after that substitution but now we also know
 * that our key is not in use as a placeholder. As this step does not modify our own key or value we know that
 * it cannot introduce new self-recursion either.
 */

pub(super) struct TypeStore {
    values: HashMap<Key,TypeConstraint>,
    uses_placeholder: HashMap<Key,HashSet<Key>>
}

impl TypeStore {
    pub(super) fn new() -> TypeStore {
        TypeStore {
            values: HashMap::new(),
            uses_placeholder: HashMap::new()
        }
    }

    fn ensure_not_recursive(&self, key: &Key, constraint: &ExpressionConstraint) -> Result<(),String> {
        if let Some(placeholder) = constraint.get_placeholder() {
            if placeholder == key {
                return Err(format!("recursive type {:?}",constraint));
            }
        }
        return Ok(())
    }

    fn set(&mut self, key: &Key, constraint: &TypeConstraint) {
        if let Some(old_value) = self.values.get(key) {
            if let Some(placeholder) = old_value.get_placeholder() {
                self.uses_placeholder.get_mut(placeholder).unwrap().remove(key);
            }
        }
        self.values.insert(key.clone(),constraint.clone());
        if let Some(placeholder) = constraint.get_placeholder() {
            self.uses_placeholder.entry(placeholder.clone()).or_insert_with(|| HashSet::new()).insert(key.clone());
        }
    }

    fn unify(&self, a: &ExpressionConstraint, b: &ExpressionConstraint) -> Result<Option<(Key,ExpressionConstraint)>,()> {
        match (a,b) {
            (ExpressionConstraint::Base(a),ExpressionConstraint::Base(b)) if a==b => Ok(None),
            (ExpressionConstraint::Vec(a),ExpressionConstraint::Vec(b)) => self.unify(a,b),
            (ExpressionConstraint::Placeholder(a),ExpressionConstraint::Placeholder(b)) if a==b => Ok(None),
            (ExpressionConstraint::Placeholder(a),x) => Ok(Some((a.clone(),x.clone()))),
            (x,ExpressionConstraint::Placeholder(a)) => Ok(Some((a.clone(),x.clone()))),
            _ => Err(())
        }
    }

    fn try_unify(&self, a: &ExpressionConstraint, b: &ExpressionConstraint) -> Result<Option<(Key,ExpressionConstraint)>,String> {
        self.unify(a,b).map_err(|_| format!("Cannout unify {:?} and {:?}",a,b))
    }

    fn replace_placeholder(&mut self, key: &Key, constraint: &TypeConstraint) -> Result<(),String> {
        if let Some(targets) = self.uses_placeholder.get(key) {
            for target in targets.clone().iter() {
                let old_value = self.values.get(target).unwrap();
                if old_value.is_reference() != constraint.is_reference() {
                    return Err(format!("Cannot unify reference and non-reference"));
                }
                let new_value = old_value.substitute(constraint.get_expressionconstraint());
                self.set(target,&new_value);
            }
        }
        Ok(())
    }

    fn apply_unification(&mut self, a: &TypeConstraint, b: &TypeConstraint) -> Result<(),String> {
        if a.is_reference() != b.is_reference() {
            return Err(format!("Cannot unify reference and non-reference"));
        }
        if let Some((key,constraint)) = self.try_unify(a.get_expressionconstraint(),b.get_expressionconstraint())? {
            if a.is_reference() {
                self.replace_placeholder(&key,&TypeConstraint::Reference(constraint))?;
            } else {
                self.replace_placeholder(&key,&TypeConstraint::NonReference(constraint))?;
            }
        }
        Ok(())
    }

    pub(super) fn add(&mut self, key: &Key, constraint: &TypeConstraint) -> Result<(),String> {
        print!("trying to add {:?} as {:?}\n",key,constraint);
        let mut constraint = constraint.clone();
        /* substitute expression to ensure store is naive to our placeholder as a key */
        if let Some(placeholder) = constraint.get_placeholder() {
            if let Some(expression) = self.values.get(placeholder).cloned() {
                if constraint.is_reference() != expression.is_reference() {
                    return Err(format!("Cannot unify reference and non-reference"));
                }
                constraint = constraint.substitute(&expression.get_expressionconstraint());
            }
        }
        print!("after applying existing rules, trying to add {:?} as {:?}\n",key,constraint);
        if let Some(existing) = self.values.get(key).cloned() {
            /* key present, unify */
            print!("Already exists so trying to unify with present value of {:?}\n",existing);
            self.apply_unification(&existing.clone(),&constraint)?;
        } else {
            self.ensure_not_recursive(key,&constraint.get_expressionconstraint())?;
            /* new key: substitute current uses of placeholder with new value */
            self.replace_placeholder(key,&constraint)?;
            /* add */
            self.set(key,&constraint);
        }
        Ok(())
    }

    pub(super) fn get(&mut self, key: &Key) -> Option<RegisterType> {
        self.values.get(key).map(|t| t.to_registertype())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::types::{ BaseType, ExpressionType };

    fn x_ph(num: usize) -> ExpressionConstraint {
        ExpressionConstraint::Placeholder(Key::External(num))
    }

    fn x_base(base: BaseType) -> ExpressionConstraint {
        ExpressionConstraint::Base(base)
    }

    fn x_vec(inner: ExpressionConstraint) -> ExpressionConstraint {
        ExpressionConstraint::Vec(Box::new(inner))
    }

    #[test]
    fn failed_unify() {
        let mut ts = TypeStore::new();
        ts.add(&Key::External(0),&TypeConstraint::NonReference(x_base(BaseType::NumberType))).expect("A");
        ts.add(&Key::External(0),&TypeConstraint::NonReference(x_base(BaseType::BooleanType))).expect_err("B");
    }

    #[test]
    fn recursive() {
        let mut ts = TypeStore::new();
        ts.add(&Key::External(0),&TypeConstraint::NonReference(x_ph(1))).expect("C");
        ts.add(&Key::External(1),&TypeConstraint::NonReference(x_ph(0))).expect_err("D");
        ts.add(&Key::External(1),&TypeConstraint::NonReference(x_vec(x_ph(0)))).expect_err("E");
    }

    #[test]
    fn identity() {
        let mut ts = TypeStore::new();
        ts.add(&Key::External(0),&TypeConstraint::NonReference(x_ph(1))).expect("F");
        ts.add(&Key::External(0),&TypeConstraint::NonReference(x_ph(1))).expect("G");
    }

    #[test]
    fn typestore_smoke() {
        let mut ts = TypeStore::new();
        ts.add(&Key::External(1),&TypeConstraint::NonReference(x_vec(x_ph(0)))).expect("H");
        ts.add(&Key::External(2),&TypeConstraint::NonReference(x_vec(x_vec(x_ph(0))))).expect("I");
        ts.add(&Key::External(3),&TypeConstraint::NonReference(x_ph(0))).expect("J");
        assert_eq!(RegisterType::NonReference(ExpressionType::Vec(Box::new(ExpressionType::Any))),ts.get(&Key::External(1)).expect("K"));
        assert_eq!(RegisterType::NonReference(ExpressionType::Vec(Box::new(ExpressionType::Vec(Box::new(ExpressionType::Any))))),ts.get(&Key::External(2)).expect("L"));
        assert_eq!(RegisterType::NonReference(ExpressionType::Any),ts.get(&Key::External(3)).expect("M"));
    }

    #[test]
    fn typestore_refnonref() {
        let mut ts = TypeStore::new();
        ts.add(&Key::External(0),&TypeConstraint::NonReference(x_ph(1))).expect("N");
        ts.add(&Key::External(0),&TypeConstraint::Reference(x_ph(1))).expect_err("O");

    }
}
