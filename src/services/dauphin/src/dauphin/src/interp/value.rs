use std::fmt::Display;
use std::iter::{ Iterator };
use std::str::from_utf8;

const MAX_USIZE : usize = 9007199254740991;

fn indexes_to_numbers(data: &mut Vec<usize>) -> Result<Vec<f64>,String> {
    data.drain(..).map(|x| {
        if x <= MAX_USIZE {
            Ok(x as f64)
        } else {
            Err(format!("Cannot convert {:?} to number",x))
        }
    }).collect()
}

fn numbers_to_indexes(data: &mut Vec<f64>) -> Result<Vec<usize>,String> {
    data.drain(..).map(|x| {
        if x >= 0. && x <= MAX_USIZE as f64 {
            Ok(x as usize)
        } else {
            Err(format!("Cannot convert {:?} to index",x))
        }
    }).collect()
}

fn boolean_to_numbers(data: &mut Vec<bool>) -> Result<Vec<f64>,String> {
    data.drain(..).map(|x| {
        Ok(if x { 1. } else { 0. })
    }).collect()
}

fn boolean_to_indexes(data: &mut Vec<bool>) -> Result<Vec<usize>,String> {
    Ok(data.drain(..).map(|x| { if x { 1 } else { 0 } }).collect())
}

fn numbers_to_boolean(data: &mut Vec<f64>) -> Result<Vec<bool>,String> {
    Ok(data.drain(..).map(|x| { x != 0. }).collect())
}

fn indexes_to_boolean(data: &mut Vec<usize>) -> Result<Vec<bool>,String> {
    Ok(data.drain(..).map(|x| { x != 0 }).collect())
}

fn strings_to_boolean(data: &mut Vec<String>) -> Result<Vec<bool>,String> {
    Ok(data.drain(..).map(|x| {
        x!=""
    }).collect())
}

fn bytes_to_boolean(data: &mut Vec<Vec<u8>>) -> Result<Vec<bool>,String> {
    Ok(data.drain(..).map(|x| {
        x.len() > 0
    }).collect())
}

fn display_to_strings<T>(data: &mut Vec<T>) -> Result<Vec<String>,String> where T: Display {
    Ok(data.drain(..).map(|x| {
        format!("{}",x)
    }).collect())
}

fn bytes_to_strings(data: &mut Vec<Vec<u8>>) -> Result<Vec<String>,String> {
    data.drain(..).map(|x| {
        from_utf8(&x).map(|x| x.to_string()).map_err(|x| format!("bad utf8 in conversion"))
    }).collect()
}

fn strings_to_bytes(data: &mut Vec<String>) -> Result<Vec<Vec<u8>>,String> {
    Ok(data.drain(..).map(|x| {
        x.as_bytes().to_vec()
    }).collect())
}

fn bytes_to_indexes(data: &mut Vec<Vec<u8>>) -> Result<Vec<usize>,String> {
    data.drain(..).map(|x| {
        if x.len() > 0 {
            Ok(x[0] as usize)
        } else {
            Err(format!("cannot convert {:?} into index",x))
        }
    }).collect()
}

fn indexes_to_bytes(data: &mut Vec<usize>) -> Result<Vec<Vec<u8>>,String> {
    data.drain(..).map(|x| {
        if x < 256 {
            Ok(vec![x as u8])
        } else {
            Err(format!("cannot convert {:?} into bytes",x))
        }
    }).collect()
}

#[derive(Clone,Debug)]
pub enum InterpValue {
    Empty,
    Numbers(Vec<f64>),
    Indexes(Vec<usize>),
    Boolean(Vec<bool>),
    Strings(Vec<String>),
    Bytes(Vec<Vec<u8>>),
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

impl InterpValue {
    pub fn len(&self) -> usize {
        match self {
            InterpValue::Empty => 0,
            InterpValue::Numbers(v) => v.len(),
            InterpValue::Indexes(v) => v.len(),
            InterpValue::Boolean(v) => v.len(),
            InterpValue::Strings(v) => v.len(),
            InterpValue::Bytes(v) => v.len(),
        }
    }

    fn get_natural(&self) -> InterpNatural {
        match self {
            InterpValue::Empty => InterpNatural::Empty,
            InterpValue::Numbers(_) => InterpNatural::Numbers,
            InterpValue::Indexes(_) => InterpNatural::Indexes,
            InterpValue::Boolean(_) => InterpNatural::Boolean,
            InterpValue::Strings(_) => InterpNatural::Strings,
            InterpValue::Bytes(_) => InterpNatural::Bytes,
        }
    }

    fn coerce(&mut self, natural: InterpNatural) -> Result<(),String> {
        match natural {
            InterpNatural::Empty => { *self = InterpValue::Empty; Ok(()) },
            InterpNatural::Numbers => self.coerce_numbers(),
            InterpNatural::Indexes => self.coerce_indexes(),
            InterpNatural::Boolean => self.coerce_boolean(),
            InterpNatural::Strings => self.coerce_strings(),
            InterpNatural::Bytes => self.coerce_bytes()
        }
    }

    fn coerce_numbers(&mut self) -> Result<(),String> {
        match self {
            InterpValue::Empty => { *self = InterpValue::Numbers(vec![]); },
            InterpValue::Numbers(_) => {},
            InterpValue::Indexes(n) => { *self = InterpValue::Numbers(indexes_to_numbers(n)?); },
            InterpValue::Boolean(n) => { *self = InterpValue::Numbers(boolean_to_numbers(n)?); },
            InterpValue::Strings(n) => { *self = InterpValue::Numbers(boolean_to_numbers(&mut strings_to_boolean(n)?)?); },
            InterpValue::Bytes(n) => { *self = InterpValue::Numbers(indexes_to_numbers(&mut bytes_to_indexes(n)?)?); },
        }
        Ok(())
    }

    fn coerce_indexes(&mut self) -> Result<(),String> {
        match self {
            InterpValue::Empty => { *self = InterpValue::Indexes(vec![]); },
            InterpValue::Numbers(n) => { *self = InterpValue::Indexes(numbers_to_indexes(n)?); },
            InterpValue::Indexes(_) => {},
            InterpValue::Boolean(n) => { *self = InterpValue::Indexes(boolean_to_indexes(n)?); },
            InterpValue::Strings(n) => { *self = InterpValue::Indexes(boolean_to_indexes(&mut strings_to_boolean(n)?)?); },
            InterpValue::Bytes(n) => { *self = InterpValue::Indexes(bytes_to_indexes(n)?); },
        }
        Ok(())
    }

    fn coerce_boolean(&mut self) -> Result<(),String> {
        match self {
            InterpValue::Empty => { *self = InterpValue::Boolean(vec![]); },
            InterpValue::Numbers(n) => { *self = InterpValue::Boolean(numbers_to_boolean(n)?); },
            InterpValue::Indexes(n) => { *self = InterpValue::Boolean(indexes_to_boolean(n)?); },
            InterpValue::Boolean(_) => {},
            InterpValue::Strings(n) => { *self = InterpValue::Boolean(strings_to_boolean(n)?); },
            InterpValue::Bytes(n) => { *self = InterpValue::Boolean(bytes_to_boolean(n)?); },
        }
        Ok(())
    }

    fn coerce_strings(&mut self) -> Result<(),String> {
        match self {
            InterpValue::Empty => { *self = InterpValue::Strings(vec![]); },
            InterpValue::Numbers(n) => { *self = InterpValue::Strings(display_to_strings(n)?); },
            InterpValue::Indexes(n) => { *self = InterpValue::Strings(display_to_strings(n)?); },
            InterpValue::Boolean(n) => { *self = InterpValue::Strings(display_to_strings(n)?); },
            InterpValue::Strings(_) => {},
            InterpValue::Bytes(n) => { *self = InterpValue::Strings(bytes_to_strings(n)?); },
        }
        Ok(())
    }

    fn coerce_bytes(&mut self) -> Result<(),String> {
        match self {
            InterpValue::Empty => { *self = InterpValue::Bytes(vec![]); },
            InterpValue::Numbers(n) => { *self = InterpValue::Bytes(indexes_to_bytes(&mut numbers_to_indexes(n)?)?); },
            InterpValue::Indexes(n) => { *self = InterpValue::Bytes(indexes_to_bytes(n)?); },
            InterpValue::Boolean(n) => { *self = InterpValue::Bytes(indexes_to_bytes(&mut boolean_to_indexes(n)?)?); },
            InterpValue::Strings(n) => { *self = InterpValue::Bytes(strings_to_bytes(n)?); },
            InterpValue::Bytes(_) => {}
        }
        Ok(())
    }

    pub fn as_numbers(&mut self) -> Result<&mut Vec<f64>,String> {
        self.coerce_numbers()?;
        if let InterpValue::Numbers(ref mut n) = self {
            Ok(n)
        } else {
            Err("cannot coerce".to_string())
        }
    }

    pub fn as_indexes(&mut self) -> Result<&mut Vec<usize>,String> {
        self.coerce_indexes()?;
        if let InterpValue::Indexes(ref mut n) = self {
            Ok(n)
        } else {
            Err("cannot coerce".to_string())
        }
    }

    pub fn as_boolean(&mut self) -> Result<&mut Vec<bool>,String> {
        self.coerce_boolean()?;
        if let InterpValue::Boolean(ref mut n) = self {
            Ok(n)
        } else {
            Err("cannot coerce".to_string())
        }
    }

    pub fn as_strings(&mut self) -> Result<&mut Vec<String>,String> {
        self.coerce_strings()?;
        if let InterpValue::Strings(ref mut n) = self {
            Ok(n)
        } else {
            Err("cannot coerce".to_string())
        }
    }

    pub fn as_bytes(&mut self) -> Result<&mut Vec<Vec<u8>>,String> {
        self.coerce_bytes()?;
        if let InterpValue::Bytes(ref mut n) = self {
            Ok(n)
        } else {
            Err("cannot coerce".to_string())
        }
    }
}
