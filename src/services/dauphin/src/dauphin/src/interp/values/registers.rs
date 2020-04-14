use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::model::Register;
use super::supercow::{ SuperCow, SuperCowCommit };
use super::value::{ InterpValue, InterpValueNumbers, InterpValueIndexes, InterpValueBoolean, InterpValueStrings, InterpValueBytes };

pub struct RegisterFile {
    values: RefCell<HashMap<Register,Rc<RefCell<SuperCow<InterpValue>>>>>,
    commits: Vec<Rc<RefCell<dyn SuperCowCommit>>>
}

/* traits would have been cleaner as an impl, but this gives more ergonomic API */
macro_rules! accessors {
    ($self:ident,$wrapper:ident,$to_exc:ident,$to_shared:ident,$type:ty,$get:ident,$coerce:ident,$take:ident) => {
        pub fn $get(&$self, register: &Register) -> Result<$wrapper,String> {
            Ok(InterpValue::$to_shared(&$self.get(register).borrow().get_shared()?)?.0)
        }

        pub fn $coerce(&mut $self, register: &Register) -> Result<$wrapper,String> {
            let v = InterpValue::$to_shared(&$self.get(register).borrow().get_shared()?)?;
            if let Some(v) = v.1 {
                $self.write_rc(register,v);
            }
            Ok(v.0)
        }

        pub fn $take(&mut $self, register: &Register) -> Result<Vec<$type>,String> {
            Ok($self.get(register).borrow_mut().get_exclusive()?.$to_exc()?)
        }
    };
}

impl RegisterFile {
    pub fn new() -> RegisterFile {
        RegisterFile {
            values: RefCell::new(HashMap::new()),
            commits: Vec::new()
        }
    }

    pub fn write_rc(&mut self, register: &Register, value: Rc<InterpValue>) {
        let cow = self.get(register);
        cow.borrow_mut().set_rc(value);
        self.commits.push(cow);
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
        self.commits.push(cow);
    }

    pub fn commit(&mut self) {
        for commit in self.commits.drain(..) {
            commit.borrow_mut().commit();
        }
    }

    pub fn copy(&mut self, dst: &Register, src: &Register) -> Result<(),String> {
        if src == dst { return Ok(()); }
        let src = self.get(src);
        let dst = self.get(dst);
        dst.borrow_mut().copy(&src.borrow())?;
        self.commits.push(dst);
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
        Ok((reg.to_string(),self.get(reg).borrow().get_shared()?.dump()?))
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
    use crate::model::RegisterAllocator;

    #[test]
    fn registers_smoke() {
        let alloc = RegisterAllocator::new();
        let r0 = alloc.allocate();
        let r1 = alloc.allocate();
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
        let alloc = RegisterAllocator::new();
        let r0 = alloc.allocate();
        let r1 = alloc.allocate();
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
        let alloc = RegisterAllocator::new();
        let r0 = alloc.allocate();
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
