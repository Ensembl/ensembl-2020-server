use std::cell::{ Ref, RefCell, RefMut };
use std::fmt::Display;
use std::iter::{ Iterator };
use std::ops::Deref;
use std::rc::Rc;
use std::str::from_utf8;
use owning_ref::{ OwningRef, RefRef };

const MAX_USIZE : usize = 9007199254740991;

pub struct ReadOnlyValues<T>(Rc<RefCell<Vec<T>>>);

impl<T> ReadOnlyValues<T> {
    pub fn borrow(&self) -> Ref<Vec<T>> {
        self.0.borrow()
    }
}

pub struct ReadWriteValues<T>(Rc<RefCell<Vec<T>>>);

impl<T> ReadWriteValues<T> {
    pub fn borrow(&self) -> Ref<Vec<T>> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<Vec<T>> {
        self.0.borrow_mut()
    }
}

fn indexes_to_numbers(data: &Vec<usize>) -> Result<Vec<f64>,String> {
    data.iter().map(|x| {
        if *x <= MAX_USIZE {
            Ok(*x as f64)
        } else {
            Err(format!("Cannot convert {:?} to number",x))
        }
    }).collect()
}

fn numbers_to_indexes(data: &Vec<f64>) -> Result<Vec<usize>,String> {
    data.iter().map(|x| {
        if *x >= 0. && *x <= MAX_USIZE as f64 {
            Ok(*x as usize)
        } else {
            Err(format!("Cannot convert {:?} to index",x))
        }
    }).collect()
}

fn boolean_to_numbers(data: &Vec<bool>) -> Result<Vec<f64>,String> {
    data.iter().map(|x| {
        Ok(if *x { 1. } else { 0. })
    }).collect()
}

fn boolean_to_indexes(data: &Vec<bool>) -> Result<Vec<usize>,String> {
    Ok(data.iter().map(|x| { if *x { 1 } else { 0 } }).collect())
}

fn numbers_to_boolean(data: &Vec<f64>) -> Result<Vec<bool>,String> {
    Ok(data.iter().map(|x| { *x != 0. }).collect())
}

fn indexes_to_boolean(data: &Vec<usize>) -> Result<Vec<bool>,String> {
    Ok(data.iter().map(|x| { *x != 0 }).collect())
}

fn strings_to_boolean(data: &Vec<String>) -> Result<Vec<bool>,String> {
    Ok(data.iter().map(|x| {
        x!=""
    }).collect())
}

fn bytes_to_boolean(data: &Vec<Vec<u8>>) -> Result<Vec<bool>,String> {
    Ok(data.iter().map(|x| {
        x.len() > 0
    }).collect())
}

fn display_to_strings<T>(data: &Vec<T>) -> Result<Vec<String>,String> where T: Display {
    Ok(data.iter().map(|x| {
        format!("{}",x)
    }).collect())
}

fn bytes_to_strings(data: &Vec<Vec<u8>>) -> Result<Vec<String>,String> {
    data.iter().map(|x| {
        from_utf8(&x).map(|x| x.to_string()).map_err(|x| format!("bad utf8 in conversion"))
    }).collect()
}

fn strings_to_bytes(data: &Vec<String>) -> Result<Vec<Vec<u8>>,String> {
    Ok(data.iter().map(|x| {
        x.as_bytes().to_vec()
    }).collect())
}

fn bytes_to_indexes(data: &Vec<Vec<u8>>) -> Result<Vec<usize>,String> {
    data.iter().map(|x| {
        if x.len() > 0 {
            Ok(x[0] as usize)
        } else {
            Err(format!("cannot convert {:?} into index",x))
        }
    }).collect()
}

fn indexes_to_bytes(data: &Vec<usize>) -> Result<Vec<Vec<u8>>,String> {
    data.iter().map(|x| {
        if *x < 256 {
            Ok(vec![*x as u8])
        } else {
            Err(format!("cannot convert {:?} into bytes",x))
        }
    }).collect()
}

#[derive(Clone,Debug)]
pub enum InterpValueData {
    Empty,
    Numbers(Rc<RefCell<Vec<f64>>>),
    Indexes(Rc<RefCell<Vec<usize>>>),
    Boolean(Rc<RefCell<Vec<bool>>>),
    Strings(Rc<RefCell<Vec<String>>>),
    Bytes(Rc<RefCell<Vec<Vec<u8>>>>),
}

fn copy_branch<T>(data: &Rc<RefCell<Vec<T>>>) -> Rc<RefCell<Vec<T>>> where T: Clone {
    Rc::new(RefCell::new(data.borrow().to_vec()))
}

#[derive(Clone,Debug)]
pub enum InterpNatural {
    Empty,
    Numbers,
    Indexes,
    Boolean,
    Strings,
    Bytes
}

impl InterpValueData {
    pub fn new_numbers(values: Vec<f64>) -> InterpValueData {
        InterpValueData::Numbers(Rc::new(RefCell::new(values)))
    }

    pub fn copy(&self) -> InterpValueData {
        match self {
            InterpValueData::Empty => InterpValueData::Empty,
            InterpValueData::Numbers(n) => InterpValueData::Numbers(copy_branch(n)),
            InterpValueData::Indexes(n) => InterpValueData::Indexes(copy_branch(n)),
            InterpValueData::Boolean(n) => InterpValueData::Boolean(copy_branch(n)),
            InterpValueData::Strings(n) => InterpValueData::Strings(copy_branch(n)),
            InterpValueData::Bytes(n) => InterpValueData::Bytes(copy_branch(n)),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            InterpValueData::Empty => 0,
            InterpValueData::Numbers(v) => v.borrow().len(),
            InterpValueData::Indexes(v) => v.borrow().len(),
            InterpValueData::Boolean(v) => v.borrow().len(),
            InterpValueData::Strings(v) => v.borrow().len(),
            InterpValueData::Bytes(v) => v.borrow().len(),
        }
    }

    pub fn get_natural(&self) -> InterpNatural {
        match self {
            InterpValueData::Empty => InterpNatural::Empty,
            InterpValueData::Numbers(_) => InterpNatural::Numbers,
            InterpValueData::Indexes(_) => InterpNatural::Indexes,
            InterpValueData::Boolean(_) => InterpNatural::Boolean,
            InterpValueData::Strings(_) => InterpNatural::Strings,
            InterpValueData::Bytes(_) => InterpNatural::Bytes,
        }
    }

    fn coerce_numbers(&self) -> Result<Rc<RefCell<Vec<f64>>>,String> {
        Ok(match self {
            InterpValueData::Empty => Rc::new(RefCell::new(vec![])),
            InterpValueData::Numbers(n) => n.clone(),
            InterpValueData::Indexes(n) => Rc::new(RefCell::new(indexes_to_numbers(&n.borrow())?)),
            InterpValueData::Boolean(n) => Rc::new(RefCell::new(boolean_to_numbers(&n.borrow())?)),
            InterpValueData::Strings(n) => Rc::new(RefCell::new(boolean_to_numbers(&strings_to_boolean(&n.borrow())?)?)),
            InterpValueData::Bytes(n) => Rc::new(RefCell::new(indexes_to_numbers(&bytes_to_indexes(&n.borrow())?)?)),
        })
    }

    fn coerce_indexes(&self) -> Result<Rc<RefCell<Vec<usize>>>,String> {
        Ok(match self {
            InterpValueData::Empty => Rc::new(RefCell::new(vec![])),
            InterpValueData::Numbers(n) => Rc::new(RefCell::new(numbers_to_indexes(&n.borrow())?)),
            InterpValueData::Indexes(n) => n.clone(),
            InterpValueData::Boolean(n) => Rc::new(RefCell::new(boolean_to_indexes(&n.borrow())?)),
            InterpValueData::Strings(n) => Rc::new(RefCell::new(boolean_to_indexes(&mut strings_to_boolean(&n.borrow())?)?)),
            InterpValueData::Bytes(n) => Rc::new(RefCell::new(bytes_to_indexes(&n.borrow())?)),
        })
    }

    fn coerce_boolean(&self) -> Result<Rc<RefCell<Vec<bool>>>,String> {
        Ok(match self {
            InterpValueData::Empty => Rc::new(RefCell::new(vec![])),
            InterpValueData::Numbers(n) => Rc::new(RefCell::new(numbers_to_boolean(&n.borrow())?)),
            InterpValueData::Indexes(n) => Rc::new(RefCell::new(indexes_to_boolean(&n.borrow())?)),
            InterpValueData::Boolean(n) => n.clone(),
            InterpValueData::Strings(n) => Rc::new(RefCell::new(strings_to_boolean(&n.borrow())?)),
            InterpValueData::Bytes(n) => Rc::new(RefCell::new(bytes_to_boolean(&n.borrow())?)),
        })
    }

    fn coerce_strings(&self) -> Result<Rc<RefCell<Vec<String>>>,String> {
        Ok(match self {
            InterpValueData::Empty => Rc::new(RefCell::new(vec![])),
            InterpValueData::Numbers(n) => Rc::new(RefCell::new(display_to_strings(&n.borrow())?)),
            InterpValueData::Indexes(n) => Rc::new(RefCell::new(display_to_strings(&n.borrow())?)),
            InterpValueData::Boolean(n) => Rc::new(RefCell::new(display_to_strings(&n.borrow())?)),
            InterpValueData::Strings(n) => n.clone(),
            InterpValueData::Bytes(n) => Rc::new(RefCell::new(bytes_to_strings(&n.borrow())?))
        })
    }

    fn coerce_bytes(&self) -> Result<Rc<RefCell<Vec<Vec<u8>>>>,String> {
        Ok(match self {
            InterpValueData::Empty => Rc::new(RefCell::new(vec![])),
            InterpValueData::Numbers(n) => Rc::new(RefCell::new(indexes_to_bytes(&mut numbers_to_indexes(&n.borrow())?)?)),
            InterpValueData::Indexes(n) => Rc::new(RefCell::new(indexes_to_bytes(&n.borrow())?)),
            InterpValueData::Boolean(n) => Rc::new(RefCell::new(indexes_to_bytes(&mut boolean_to_indexes(&n.borrow())?)?)),
            InterpValueData::Strings(n) => Rc::new(RefCell::new(strings_to_bytes(&n.borrow())?)),
            InterpValueData::Bytes(n) => n.clone()
        })
    }

    pub fn read_numbers(&self) -> Result<ReadOnlyValues<f64>,String> {
        Ok(ReadOnlyValues(self.coerce_numbers()?.clone()))
    }

    pub fn write_numbers(&mut self) -> Result<ReadWriteValues<f64>,String> {
        *self = InterpValueData::Numbers(self.coerce_numbers()?);
        Ok(ReadWriteValues(self.coerce_numbers()?.clone()))
    }

    pub fn read_indexes(&self) -> Result<ReadOnlyValues<usize>,String> {
        Ok(ReadOnlyValues(self.coerce_indexes()?.clone()))
    }

    pub fn write_indexes(&mut self) -> Result<ReadWriteValues<usize>,String> {
        *self = InterpValueData::Indexes(self.coerce_indexes()?);
        Ok(ReadWriteValues(self.coerce_indexes()?.clone()))
    }

    pub fn read_boolean(&self) -> Result<ReadOnlyValues<bool>,String> {
        Ok(ReadOnlyValues(self.coerce_boolean()?.clone()))
    }

    pub fn write_boolean(&mut self) -> Result<ReadWriteValues<bool>,String> {
        *self = InterpValueData::Boolean(self.coerce_boolean()?);
        Ok(ReadWriteValues(self.coerce_boolean()?.clone()))
    }

    pub fn read_strings(&self) -> Result<ReadOnlyValues<String>,String> {
        Ok(ReadOnlyValues(self.coerce_strings()?.clone()))
    }

    pub fn write_strings(&mut self) -> Result<ReadWriteValues<String>,String> {
        *self = InterpValueData::Strings(self.coerce_strings()?);
        Ok(ReadWriteValues(self.coerce_strings()?.clone()))
    }

    pub fn read_bytes(&self) -> Result<ReadOnlyValues<Vec<u8>>,String> {
        Ok(ReadOnlyValues(self.coerce_bytes()?.clone()))
    }

    pub fn write_bytes(&mut self) -> Result<ReadWriteValues<Vec<u8>>,String> {
        *self = InterpValueData::Bytes(self.coerce_bytes()?);
        Ok(ReadWriteValues(self.coerce_bytes()?.clone()))
    }
}
