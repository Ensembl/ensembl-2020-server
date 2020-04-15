use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use serde_cbor::Value as CborValue;

#[derive(Clone,Copy,Hash,PartialEq,Eq,PartialOrd,Ord)]
pub struct Register(pub usize);

impl Register {
    pub fn deserialize(v: &CborValue) -> Result<Register,String> {
        if let CborValue::Integer(r) = v {
            Ok(Register(*r as usize))
        } else {
            Err("bad cbor, expected register".to_string())
        }
    }

    pub fn serialize(&self) -> CborValue {
        CborValue::Integer(self.0 as i128)
    }
}

impl fmt::Debug for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"%{}",self.0)?;
        Ok(())
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"%{}",self.0)?;
        Ok(())
    }
}

#[derive(Debug)]
struct RegisterAllocatorImpl {
    index: usize
}

impl RegisterAllocatorImpl {
    fn new() -> RegisterAllocatorImpl {
        RegisterAllocatorImpl {
            index: 0
        }
    }

    fn allocate(&mut self) -> Register {
        self.index += 1;
        Register(self.index)
    }
}

#[derive(Clone,Debug)]
pub struct RegisterAllocator(Rc<RefCell<RegisterAllocatorImpl>>);

impl RegisterAllocator {
    pub fn new() -> RegisterAllocator {
        RegisterAllocator(Rc::new(RefCell::new(RegisterAllocatorImpl::new())))
    }

    pub fn allocate(&self) -> Register {
        self.0.borrow_mut().allocate().clone()
    }
}
