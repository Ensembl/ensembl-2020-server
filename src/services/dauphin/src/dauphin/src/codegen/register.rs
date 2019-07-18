use std::fmt;

#[derive(Clone,Hash,PartialEq,Eq)]
pub enum Register {
    Named(String),
    Temporary(usize)
}

impl fmt::Debug for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::Named(n) => write!(f,"%{}",n)?,
            Register::Temporary(i) => write!(f,"%:{}",i)?
        }
        Ok(())
    }
}

pub struct RegisterAllocator {
    index: usize
}

impl RegisterAllocator {
    pub fn new() -> RegisterAllocator {
        RegisterAllocator {
            index: 0
        }
    }

    pub fn allocate(&mut self) -> Register {
        self.index += 1;
        Register::Temporary(self.index)
    }
}
