use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone,Hash,PartialEq,Eq,PartialOrd,Ord)]
pub struct Register(usize);

impl fmt::Debug for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"%{}",self.0)?;
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
        Register(self.index)
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
