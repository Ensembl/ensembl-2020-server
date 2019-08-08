use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone,Hash,PartialEq,Eq,PartialOrd,Ord)]
pub enum Register {
    Named(String),
    Left(Box<Register>,String), // XXX fixme
    Temporary(usize),
}

impl fmt::Debug for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::Named(n) => write!(f,"%{}",n)?,
            Register::Left(r,v) => write!(f,"{:?}:{}",r,v)?,
            Register::Temporary(i) => write!(f,"%:{}",i)?
        }
        Ok(())
    }
}

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
        Register::Temporary(self.index)
    }
}

#[derive(Clone)]
pub struct RegisterAllocator(Rc<RefCell<RegisterAllocatorImpl>>);

impl RegisterAllocator {
    pub fn new() -> RegisterAllocator {
        RegisterAllocator(Rc::new(RefCell::new(RegisterAllocatorImpl::new())))
    }

    pub fn allocate(&self) -> Register {
        self.0.borrow_mut().allocate().clone()
    }
}
