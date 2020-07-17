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
 */

use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::replace;
use std::rc::Rc;
use crate::runtime::{ Register, SuperCow, SuperCowCommit };
use super::value::{ InterpValue, InterpValueNumbers, InterpValueIndexes, InterpValueBoolean, InterpValueStrings, InterpValueBytes };

pub struct RegisterFile {
    values: RefCell<HashMap<Register,Rc<RefCell<SuperCow<InterpValue>>>>>,
    commit_order: Vec<Register>,
    commit_value: HashMap<Register,Rc<RefCell<dyn SuperCowCommit>>>
}

/* traits would have been cleaner as an impl, but this gives more ergonomic API */
macro_rules! accessors {
    ($self:ident,$wrapper:ident,$to_exc:ident,$to_shared:ident,$type:ty,$get:ident,$coerce:ident,$take:ident) => {
        #[allow(unused)]
        pub fn $get(&$self, register: &Register) -> Result<$wrapper,String> {
            Ok(InterpValue::$to_shared(&$self.get(register).borrow().get_shared()?)?.0)
        }

        #[allow(unused)]
        pub fn $coerce(&mut $self, register: &Register) -> Result<$wrapper,String> {
            let v = InterpValue::$to_shared(&$self.get(register).borrow().get_shared()?)?;
            if let Some(v) = v.1 {
                $self.write_rc(register,v);
            }
            Ok(v.0)
        }

        #[allow(unused)]
        pub fn $take(&mut $self, register: &Register) -> Result<Vec<$type>,String> {
            Ok($self.get(register).borrow_mut().get_exclusive()?.$to_exc()?)
        }
    };
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        RegisterFile {
            values: RefCell::new(HashMap::new()),
            commit_order: vec![],
            commit_value: HashMap::new()
        }
    }

    fn add_commit(&mut self, register: &Register, cow: Rc<RefCell<dyn SuperCowCommit>>) {
        self.commit_order.push(*register);
        self.commit_value.insert(*register,cow);
    }

    pub fn write_rc(&mut self, register: &Register, value: Rc<InterpValue>) {
        let cow = self.get(register);
        cow.borrow_mut().set_rc(value);
        self.add_commit(register,cow);
    }

    pub fn get(&self, register: &Register) -> Rc<RefCell<SuperCow<InterpValue>>> {
        self.values.borrow_mut().entry(*register).or_insert_with(|| 
            Rc::new(RefCell::new(SuperCow::new(InterpValue::Empty,|x| { x.copy() })))
        ).clone()
    }

    pub fn len(&self, register: &Register) -> Result<usize,String> {
        let reg = self.get(register);
        let reg = reg.borrow().get_shared()?;
        Ok(reg.len())
    }

    pub fn write(&mut self, register: &Register, value: InterpValue) {
        let cow = self.get(register);
        cow.borrow_mut().set(value);
        self.add_commit(register,cow);
    }

    pub fn commit(&mut self) -> Vec<Register> {
        let regs = replace(&mut self.commit_order,vec![]);
        for register in &regs {
            self.commit_value[&register].borrow_mut().commit();
        }
        regs
    }

    pub fn copy(&mut self, dst: &Register, src: &Register) -> Result<(),String> {
        if src == dst { return Ok(()); }
        let srcv = self.get(src);
        let dstv = self.get(dst);
        dstv.borrow_mut().copy(&srcv.borrow())?;
        self.add_commit(dst,dstv);
        Ok(())
    }

    accessors!(self,InterpValueNumbers,to_numbers,to_rc_numbers,f64,get_numbers,coerce_numbers,take_numbers);
    accessors!(self,InterpValueIndexes,to_indexes,to_rc_indexes,usize,get_indexes,coerce_indexes,take_indexes);
    accessors!(self,InterpValueBoolean,to_boolean,to_rc_boolean,bool,get_boolean,coerce_boolean,take_boolean);
    accessors!(self,InterpValueStrings,to_strings,to_rc_strings,String,get_strings,coerce_strings,take_strings);
    accessors!(self,InterpValueBytes,to_bytes,to_rc_bytes,Vec<u8>,get_bytes,coerce_bytes,take_bytes);

    pub fn export(&self) -> Result<HashMap<Register,InterpValue>,String> {
        let mut out = HashMap::new();
        for r in self.values.borrow().iter() {
            out.insert(*r.0,r.1.borrow().get_shared()?.copy());
        }
        Ok(out)
    }

    pub fn dump(&self, reg: &Register) -> Result<(String,String),String> {
        Ok((reg.to_string(),self.get(reg).borrow().get_shared().and_then(|x| x.dump()).unwrap_or_else(|_| format!("???"))))
    }

    fn dump_one(&self, reg: &Register) -> Result<String,String> {
        let x = self.dump(reg)?;
        Ok(format!("{} = {}",x.0,x.1))
    }

    pub fn dump_many(&self, regs: &[Register]) -> Result<String,String> {
        Ok(format!("{}\n",regs.iter().map(|x| self.dump_one(x)).collect::<Result<Vec<_>,_>>()?.join("    ")))
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn registers_smoke() {
        let r0 = Register(0);
        let r1 = Register(1);
        let mut rf = RegisterFile::new();
        rf.write(&r0,InterpValue::Indexes(vec![1,2,3]));
        rf.write(&r1,InterpValue::Indexes(vec![4,5,6]));
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("A");
        assert_eq!(&vec![1,2,3],x);
        let x : &Vec<usize> = &rf.get_indexes(&r1).expect("B");
        assert_eq!(&vec![4,5,6],x);
        rf.write(&r0,InterpValue::Indexes(vec![7]));
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("C");
        assert_eq!(&vec![1,2,3],x);
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("D");
        assert_eq!(&vec![7],x);

    }

    #[test]
    fn registers_copy() {
        let r0 = Register(0);
        let r1 = Register(1);
        let mut rf = RegisterFile::new();
        rf.write(&r0,InterpValue::Indexes(vec![1,2,3]));
        rf.write(&r1,InterpValue::Indexes(vec![4,5,6]));
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r1).expect("B");
        assert_eq!(&vec![4,5,6],x);
        rf.copy(&r1,&r0).expect("A");
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r1).expect("C");
        assert_eq!(&vec![1,2,3],x);
        rf.write(&r0,InterpValue::Indexes(vec![7,8,9]));
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r1).expect("D");
        assert_eq!(&vec![1,2,3],x);
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("E");
        assert_eq!(&vec![7,8,9],x);
    }

    #[test]
    fn registers_coerce() {
        let r0 = Register(0);
        let mut rf = RegisterFile::new();
        rf.write(&r0,InterpValue::Indexes(vec![0,1,2]));
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("A");
        assert_eq!(&vec![0,1,2],x);
        let x : &Vec<bool> = &rf.get_boolean(&r0).expect("B");
        assert_eq!(&vec![false,true,true],x);
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("C");
        assert_eq!(&vec![0,1,2],x);
        let x : &Vec<bool> = &rf.coerce_boolean(&r0).expect("D");
        assert_eq!(&vec![false,true,true],x);
        rf.commit();
        let x : &Vec<usize> = &rf.get_indexes(&r0).expect("C");
        assert_eq!(&vec![0,1,1],x);
    }
}

