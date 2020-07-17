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

use std::ops::Deref;
use std::fmt::Display;
use std::iter::{ Iterator };
use std::rc::Rc;
use std::str::from_utf8;

pub const MAX_USIZE : usize = 9007199254740991;

fn print_value<T>(data: &[T]) -> String where T: std::fmt::Display {
    format!("[{}]",data.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn print_bytes<T>(data: &[Vec<T>]) -> String where T: std::fmt::Display {
    format!("[{}]",data.iter().map(|x| { print_value(x) }).collect::<Vec<_>>().join(", "))
}

pub fn to_index(value: f64) -> Option<usize> {
    if value >= 0. && value <= MAX_USIZE as f64 {
        Some(value as usize)
    } else {
        None
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

pub fn numbers_to_indexes(data: &Vec<f64>) -> Result<Vec<usize>,String> {
    data.iter().map(|x| {
        if let Some(x) = to_index(*x) {
            Ok(x)
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
        from_utf8(&x).map(|x| x.to_string()).map_err(|_| format!("bad utf8 in conversion"))
    }).collect()
}

fn strings_to_bytes(data: &Vec<String>) -> Result<Vec<Vec<u8>>,String> {
    Ok(data.iter().map(|x| {
        x.as_bytes().to_vec()
    }).collect())
}

fn bytes_to_indexes(data: &Vec<Vec<u8>>) -> Result<Vec<usize>,String> {
    if data.len() == 0 { return Ok(vec![]); }
    data.iter().map(|x| {
        if x.len() > 0 {
            Ok(x[0] as usize)
        } else {
            Err(format!("cannot convert {:?} into indexes",x))
        }
    }).collect()
}

fn indexes_to_bytes(data: &Vec<usize>) -> Result<Vec<Vec<u8>>,String> {
    if data.len() == 0 { return Ok(vec![]); }
    data.iter().map(|x| {
        if *x < 256 {
            Ok(vec![*x as u8])
        } else {
            Err(format!("cannot convert {:?} into bytes",x))
        }
    }).collect()
}

#[derive(Debug)]
pub enum InterpValue {
    Empty,
    Numbers(Vec<f64>),
    Indexes(Vec<usize>),
    Boolean(Vec<bool>),
    Strings(Vec<String>),
    Bytes(Vec<Vec<u8>>),
}

macro_rules! interp_value {
    ($type:ident,$branch: tt,$inner:ty) => {
        #[derive(Clone,Debug)]
        pub struct $type(Rc<InterpValue>);

        impl Deref for $type {
            type Target = Vec<$inner>;

            fn deref(&self) -> &Self::Target {
                if let InterpValue::$branch(n) = &*self.0 { &n } else { panic!("coercing failed") }
            }
        }
    };
}

interp_value!(InterpValueNumbers,Numbers,f64);
interp_value!(InterpValueIndexes,Indexes,usize);
interp_value!(InterpValueBoolean,Boolean,bool);
interp_value!(InterpValueStrings,Strings,String);
interp_value!(InterpValueBytes,Bytes,Vec<u8>);

#[derive(Clone,Debug)]
pub enum InterpNatural {
    Empty,
    Numbers,
    Indexes,
    Boolean,
    Strings,
    Bytes
}

macro_rules! accessor {
    ($self:ident,$branch:tt,$coercer:ident,$wrapper:ident,$type:ty,$exc:ident,$shared:ident,$coerced:ident) => {
        pub fn $exc($self) -> Result<Vec<$type>,String> {
            Ok($self.$coercer()?.unwrap_or(if let InterpValue::$branch(n) = $self { n } else { vec![] }))
        }

        pub fn $shared(self: &Rc<Self>) -> Result<($wrapper,Option<Rc<InterpValue>>),String> {
            let x = self.$coercer()?
                .map(|x| {
                    let r = Rc::new(InterpValue::$branch(x));
                    (r.clone(),Some(r))
                })
                .unwrap_or_else(|| (self.clone(),None));
            Ok(($wrapper(x.0),x.1))
        }
    };
}

impl InterpValue {
    pub fn get_natural(&self) -> InterpNatural {
        match self {
            InterpValue::Empty => InterpNatural::Empty,
            InterpValue::Numbers(_) => InterpNatural::Numbers,
            InterpValue::Indexes(_) => InterpNatural::Indexes,
            InterpValue::Boolean(_) => InterpNatural::Boolean,
            InterpValue::Strings(_) => InterpNatural::Strings,
            InterpValue::Bytes(_) => InterpNatural::Bytes,
        }
    }

    pub fn copy(&self) -> InterpValue {
        match self {
            InterpValue::Empty => InterpValue::Empty,
            InterpValue::Numbers(n) => InterpValue::Numbers(n.to_vec()),
            InterpValue::Indexes(n) => InterpValue::Indexes(n.to_vec()),
            InterpValue::Boolean(n) => InterpValue::Boolean(n.to_vec()),
            InterpValue::Strings(n) => InterpValue::Strings(n.to_vec()),
            InterpValue::Bytes(n) => InterpValue::Bytes(n.iter().map(|x| x.to_vec()).collect())
        }
    }

    pub fn len(&self) -> usize {
        match self {
            InterpValue::Empty => 0,
            InterpValue::Numbers(n) => n.len(),
            InterpValue::Indexes(n) => n.len(),
            InterpValue::Boolean(n) => n.len(),
            InterpValue::Strings(n) => n.len(),
            InterpValue::Bytes(n) => n.len()
        }
    }

    pub fn coerce_numbers(&self) -> Result<Option<Vec<f64>>,String> {
        Ok(match self {
            InterpValue::Empty => Some(vec![]),
            InterpValue::Numbers(_) => None,
            InterpValue::Indexes(n) => Some(indexes_to_numbers(&n)?),
            InterpValue::Boolean(n) => Some(boolean_to_numbers(&n)?),
            InterpValue::Strings(n) => Some(boolean_to_numbers(&strings_to_boolean(&n)?)?),
            InterpValue::Bytes(n) => Some(indexes_to_numbers(&bytes_to_indexes(&n)?)?),
        })
    }

    pub fn coerce_indexes(&self) -> Result<Option<Vec<usize>>,String> {
        Ok(match self {
            InterpValue::Empty => Some(vec![]),
            InterpValue::Numbers(n) => Some(numbers_to_indexes(&n)?),
            InterpValue::Indexes(_) => None,
            InterpValue::Boolean(n) => Some(boolean_to_indexes(&n)?),
            InterpValue::Strings(n) => Some(boolean_to_indexes(&strings_to_boolean(&n)?)?),
            InterpValue::Bytes(n) => Some(bytes_to_indexes(&n)?),
        })
    }

    pub fn coerce_boolean(&self) -> Result<Option<Vec<bool>>,String> {
        Ok(match self {
            InterpValue::Empty => Some(vec![]),
            InterpValue::Numbers(n) => Some(numbers_to_boolean(&n)?),
            InterpValue::Indexes(n) => Some(indexes_to_boolean(&n)?),
            InterpValue::Boolean(_) => None,
            InterpValue::Strings(n) => Some(strings_to_boolean(&n)?),
            InterpValue::Bytes(n) => Some(bytes_to_boolean(&n)?),
        })
    }

    pub fn coerce_strings(&self) -> Result<Option<Vec<String>>,String> {
        Ok(match self {
            InterpValue::Empty => Some(vec![]),
            InterpValue::Numbers(n) => Some(display_to_strings(&n)?),
            InterpValue::Indexes(n) => Some(display_to_strings(&n)?),
            InterpValue::Boolean(n) => Some(display_to_strings(&n)?),
            InterpValue::Strings(_) => None,
            InterpValue::Bytes(n) => Some(bytes_to_strings(&n)?),
        })
    }

    pub fn coerce_bytes(&self) -> Result<Option<Vec<Vec<u8>>>,String> {
        Ok(match self {
            InterpValue::Empty => Some(vec![]),
            InterpValue::Numbers(n) => Some(indexes_to_bytes(&numbers_to_indexes(&n)?)?),
            InterpValue::Indexes(n) => Some(indexes_to_bytes(&n)?),
            InterpValue::Boolean(n) => Some(indexes_to_bytes(&boolean_to_indexes(&n)?)?),
            InterpValue::Strings(n) => Some(strings_to_bytes(&n)?),
            InterpValue::Bytes(_) => None
        })
    }

    accessor!(self,Numbers,coerce_numbers,InterpValueNumbers,f64,to_numbers,to_rc_numbers,to_coerced_numbers);
    accessor!(self,Indexes,coerce_indexes,InterpValueIndexes,usize,to_indexes,to_rc_indexes,to_coerced_indexes);
    accessor!(self,Boolean,coerce_boolean,InterpValueBoolean,bool,to_boolean,to_rc_boolean,to_coerced_boolean);
    accessor!(self,Strings,coerce_strings,InterpValueStrings,String,to_strings,to_rc_strings,to_coerced_strings);
    accessor!(self,Bytes,coerce_bytes,InterpValueBytes,Vec<u8>,to_bytes,to_rc_bytes,to_coerced_bytes);

    pub fn dump(self: &Rc<Self>) -> Result<String,String> {
        Ok(match self.get_natural() {
            InterpNatural::Empty => { String::new() },
            InterpNatural::Numbers => { print_value(&self.to_rc_numbers()?.0) },
            InterpNatural::Indexes => { print_value(&self.to_rc_indexes()?.0) },
            InterpNatural::Boolean => { print_value(&self.to_rc_boolean()?.0) },
            InterpNatural::Strings => { print_value(&self.to_rc_strings()?.0) },
            InterpNatural::Bytes => { print_bytes(&self.to_rc_bytes()?.0) },
        })     
    }
}


#[cfg(test)]
mod test {
    use super::*;

    // XXX general utility
    macro_rules! assert_branch {
        ($b:pat, $v:expr) => (
            if let $b = $v {} else { panic!("wrong branch"); }
        );
    }

    macro_rules! vecstring {
        ($($x:expr),*) => {
            vec![
                $($x.to_string()),*
            ]
        };
    }

    fn array_eq<T>(a: &[T], b: &[T]) where T: PartialEq+std::fmt::Debug {
        assert_eq!(a,b);
    }

    #[test]
    fn value_indexes() {
        let v = Rc::new(InterpValue::Indexes(vec![0,1,2,3]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,2,3],&d.0);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.,2.,3.],&d.0);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d.0);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["0","1","2","3"],&d.0);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![0],vec![1],vec![2],vec![3]],&d.0);
        assert_branch!(InterpNatural::Indexes,v.get_natural());
        assert_eq!(4,v.len());
    }

    #[test]
    fn value_numbers() {
        let v = Rc::new(InterpValue::Numbers(vec![0.,1.,2.,3.]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,2,3],&d.0);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.,2.,3.],&d.0);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d.0);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["0","1","2","3"],&d.0);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![0],vec![1],vec![2],vec![3]],&d.0);
        assert_branch!(InterpNatural::Numbers,v.get_natural());
        assert_eq!(4,v.len());
    }

    #[test]
    fn value2_boolean() {
        let v = Rc::new(InterpValue::Boolean(vec![false,true]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1],&d.0);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.],&d.0);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true],&d.0);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["false","true"],&d.0);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![0_u8],vec![1]],&d.0);
        assert_branch!(InterpNatural::Boolean,v.get_natural());
        assert_eq!(2,v.len());
    }

    #[test]
    fn value_string() {
        let v = Rc::new(InterpValue::Strings(vecstring!["","x","y","zz"]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,1,1],&d.0);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.,1.,1.],&d.0);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d.0);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["","x","y","zz"],&d.0);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![],vec![120],vec![121],vec![122,122]],&d.0);
        assert_branch!(InterpNatural::Strings,v.get_natural());
        assert_eq!(4,v.len());
    }

    #[test]
    fn value_bytes() {
        let v = Rc::new(InterpValue::Bytes(vec![vec![],vec![120],vec![121],vec![122,122]]));
        v.to_rc_indexes().map(|_| {}).expect_err("B");
        v.to_rc_numbers().map(|_| {}).expect_err("C");
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d.0);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["","x","y","zz"],&d.0);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![],vec![120],vec![121],vec![122,122]],&d.0);
        assert_branch!(InterpNatural::Bytes,v.get_natural());
        assert_eq!(4,v.len());
    }

    fn range_f64_check(number: f64) -> bool {
        let v = Rc::new(InterpValue::Numbers(vec![number]));
        let w = v.to_rc_indexes();
        if let Ok(ref w) = w {
            assert_eq!(w.0[0] as f64,number);
            assert_eq!(w.0[0],number as usize);
            let x = Rc::new(InterpValue::Indexes(vec![w.0[0] as usize]));
            let w2 = x.to_rc_numbers();
            assert!(w2.is_ok());
            assert_eq!(w2.unwrap().0[0],number);
        }
        w.is_ok()
    }

    #[test]
    fn test_range() {
        assert!(range_f64_check(0.));
        assert!(range_f64_check(1.));
        assert!(!range_f64_check(-1.));
        assert!(range_f64_check(MAX_USIZE as f64));
        assert!(!range_f64_check(MAX_USIZE as f64 + 1.));
    }

    #[test]
    fn test_write() {
        let mut v = Rc::new(InterpValue::Numbers(vec![0.,1.,2.,3.]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,2,3],&d.0);
        let mut e = Rc::get_mut(&mut v).unwrap().copy().to_numbers().expect("C");
        e[1] = -1.;
        let v = Rc::new(InterpValue::Numbers(e));
        let d = v.to_rc_numbers().expect("B");
        array_eq(&vec![0.,-1.,2.,3.],&d.0);
    }
}
