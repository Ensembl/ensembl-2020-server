use std::ops::Deref;
use std::fmt::Display;
use std::iter::{ Iterator };
use std::rc::Rc;
use std::str::from_utf8;

const MAX_USIZE : usize = 9007199254740991;

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

fn numbers_to_indexes(data: &Vec<f64>) -> Result<Vec<usize>,String> {
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

#[derive(Debug)]
pub enum InterpValueData {
    Empty,
    Numbers(Vec<f64>),
    Indexes(Vec<usize>),
    Boolean(Vec<bool>),
    Strings(Vec<String>),
    Bytes(Vec<Vec<u8>>),
}

macro_rules! interp_value {
    ($type:ident,$branch: tt,$inner:ty) => {
        pub struct $type(Rc<InterpValueData>);

        impl Deref for $type {
            type Target = Vec<$inner>;

            fn deref(&self) -> &Self::Target {
                if let InterpValueData::$branch(n) = &*self.0 { &n } else { panic!("coercing failed") }
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

impl InterpValueData {
    pub fn get_natural(&self) -> InterpNatural {
        match self {
            InterpValueData::Empty => InterpNatural::Empty,
            InterpValueData::Numbers(n) => InterpNatural::Numbers,
            InterpValueData::Indexes(n) => InterpNatural::Indexes,
            InterpValueData::Boolean(n) => InterpNatural::Boolean,
            InterpValueData::Strings(n) => InterpNatural::Strings,
            InterpValueData::Bytes(n) => InterpNatural::Bytes,
        }
    }

    pub fn copy(&self) -> InterpValueData {
        match self {
            InterpValueData::Empty => InterpValueData::Empty,
            InterpValueData::Numbers(n) => InterpValueData::Numbers(n.to_vec()),
            InterpValueData::Indexes(n) => InterpValueData::Indexes(n.to_vec()),
            InterpValueData::Boolean(n) => InterpValueData::Boolean(n.to_vec()),
            InterpValueData::Strings(n) => InterpValueData::Strings(n.to_vec()),
            InterpValueData::Bytes(n) => InterpValueData::Bytes(n.iter().map(|x| x.to_vec()).collect())
        }
    }

    pub fn len(&self) -> usize {
        match self {
            InterpValueData::Empty => 0,
            InterpValueData::Numbers(n) => n.len(),
            InterpValueData::Indexes(n) => n.len(),
            InterpValueData::Boolean(n) => n.len(),
            InterpValueData::Strings(n) => n.len(),
            InterpValueData::Bytes(n) => n.len()
        }
    }

    fn numbers(&self) -> Result<Option<Vec<f64>>,String> {
        Ok(match self {
            InterpValueData::Empty => Some(vec![]),
            InterpValueData::Numbers(_) => None,
            InterpValueData::Indexes(n) => Some(indexes_to_numbers(&n)?),
            InterpValueData::Boolean(n) => Some(boolean_to_numbers(&n)?),
            InterpValueData::Strings(n) => Some(boolean_to_numbers(&strings_to_boolean(&n)?)?),
            InterpValueData::Bytes(n) => Some(indexes_to_numbers(&bytes_to_indexes(&n)?)?),
        })
    }

    fn indexes(&self) -> Result<Option<Vec<usize>>,String> {
        Ok(match self {
            InterpValueData::Empty => Some(vec![]),
            InterpValueData::Numbers(n) => Some(numbers_to_indexes(&n)?),
            InterpValueData::Indexes(_) => None,
            InterpValueData::Boolean(n) => Some(boolean_to_indexes(&n)?),
            InterpValueData::Strings(n) => Some(boolean_to_indexes(&strings_to_boolean(&n)?)?),
            InterpValueData::Bytes(n) => Some(bytes_to_indexes(&n)?),
        })
    }

    fn boolean(&self) -> Result<Option<Vec<bool>>,String> {
        Ok(match self {
            InterpValueData::Empty => Some(vec![]),
            InterpValueData::Numbers(n) => Some(numbers_to_boolean(&n)?),
            InterpValueData::Indexes(n) => Some(indexes_to_boolean(&n)?),
            InterpValueData::Boolean(_) => None,
            InterpValueData::Strings(n) => Some(strings_to_boolean(&n)?),
            InterpValueData::Bytes(n) => Some(bytes_to_boolean(&n)?),
        })
    }

    fn strings(&self) -> Result<Option<Vec<String>>,String> {
        Ok(match self {
            InterpValueData::Empty => Some(vec![]),
            InterpValueData::Numbers(n) => Some(display_to_strings(&n)?),
            InterpValueData::Indexes(n) => Some(display_to_strings(&n)?),
            InterpValueData::Boolean(n) => Some(display_to_strings(&n)?),
            InterpValueData::Strings(_) => None,
            InterpValueData::Bytes(n) => Some(bytes_to_strings(&n)?),
        })
    }

    fn bytes(&self) -> Result<Option<Vec<Vec<u8>>>,String> {
        Ok(match self {
            InterpValueData::Empty => Some(vec![]),
            InterpValueData::Numbers(n) => Some(indexes_to_bytes(&numbers_to_indexes(&n)?)?),
            InterpValueData::Indexes(n) => Some(indexes_to_bytes(&n)?),
            InterpValueData::Boolean(n) => Some(indexes_to_bytes(&boolean_to_indexes(&n)?)?),
            InterpValueData::Strings(n) => Some(strings_to_bytes(&n)?),
            InterpValueData::Bytes(_) => None
        })
    }

    pub fn to_numbers(self) -> Result<Vec<f64>,String> {
        Ok(self.numbers()?.unwrap_or(if let InterpValueData::Numbers(n) = self { n } else { vec![] }))
    }

    pub fn to_rc_numbers(self: &Rc<Self>) -> Result<InterpValueNumbers,String> {
        Ok(InterpValueNumbers(self.numbers()?.map(|x| Rc::new(InterpValueData::Numbers(x))).unwrap_or_else(|| self.clone())))
    }

    pub fn to_indexes(self) -> Result<Vec<usize>,String> {
        Ok(self.indexes()?.unwrap_or(if let InterpValueData::Indexes(n) = self { n } else { vec![] }))
    }

    pub fn to_rc_indexes(self: &Rc<Self>) -> Result<InterpValueIndexes,String> {
        Ok(InterpValueIndexes(self.indexes()?.map(|x| Rc::new(InterpValueData::Indexes(x))).unwrap_or_else(|| self.clone())))
    }

    pub fn to_boolean(self) -> Result<Vec<bool>,String> {
        Ok(self.boolean()?.unwrap_or(if let InterpValueData::Boolean(n) = self { n } else { vec![] }))
    }

    pub fn to_rc_boolean(self: &Rc<Self>) -> Result<InterpValueBoolean,String> {
        Ok(InterpValueBoolean(self.boolean()?.map(|x| Rc::new(InterpValueData::Boolean(x))).unwrap_or_else(|| self.clone())))
    }

    pub fn to_strings(self) -> Result<Vec<String>,String> {
        Ok(self.strings()?.unwrap_or(if let InterpValueData::Strings(n) = self { n } else { vec![] }))
    }

    pub fn to_rc_strings(self: &Rc<Self>) -> Result<InterpValueStrings,String> {
        Ok(InterpValueStrings(self.strings()?.map(|x| Rc::new(InterpValueData::Strings(x))).unwrap_or_else(|| self.clone())))
    }

    pub fn to_bytes(self) -> Result<Vec<Vec<u8>>,String> {
        Ok(self.bytes()?.unwrap_or(if let InterpValueData::Bytes(n) = self { n } else { vec![] }))
    }

    pub fn to_rc_bytes(self: &Rc<Self>) -> Result<InterpValueBytes,String> {
        Ok(InterpValueBytes(self.bytes()?.map(|x| Rc::new(InterpValueData::Bytes(x))).unwrap_or_else(|| self.clone())))
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
        let v = Rc::new(InterpValueData::Indexes(vec![0,1,2,3]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,2,3],&d);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.,2.,3.],&d);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["0","1","2","3"],&d);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![0],vec![1],vec![2],vec![3]],&d);
        assert_branch!(InterpNatural::Indexes,v.get_natural());
        assert_eq!(4,v.len());
    }

    #[test]
    fn value_numbers() {
        let v = Rc::new(InterpValueData::Numbers(vec![0.,1.,2.,3.]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,2,3],&d);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.,2.,3.],&d);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["0","1","2","3"],&d);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![0],vec![1],vec![2],vec![3]],&d);
        assert_branch!(InterpNatural::Numbers,v.get_natural());
        assert_eq!(4,v.len());
    }

    #[test]
    fn value2_boolean() {
        let v = Rc::new(InterpValueData::Boolean(vec![false,true]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1],&d);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.],&d);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true],&d);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["false","true"],&d);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![0_u8],vec![1]],&d);
        assert_branch!(InterpNatural::Boolean,v.get_natural());
        assert_eq!(2,v.len());
    }

    #[test]
    fn value_string() {
        let v = Rc::new(InterpValueData::Strings(vecstring!["","x","y","zz"]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,1,1],&d);
        let d = v.to_rc_numbers().expect("C");
        array_eq(&vec![0.,1.,1.,1.],&d);
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["","x","y","zz"],&d);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![],vec![120],vec![121],vec![122,122]],&d);
        assert_branch!(InterpNatural::Strings,v.get_natural());
        assert_eq!(4,v.len());
    }

    #[test]
    fn value_bytes() {
        let v = Rc::new(InterpValueData::Bytes(vec![vec![],vec![120],vec![121],vec![122,122]]));
        v.to_rc_indexes().map(|_| {}).expect_err("B");
        v.to_rc_numbers().map(|_| {}).expect_err("C");
        let d = v.to_rc_boolean().expect("D");
        array_eq(&vec![false,true,true,true],&d);
        let d = v.to_rc_strings().expect("E");
        array_eq(&vecstring!["","x","y","zz"],&d);
        let d = v.to_rc_bytes().expect("F");
        array_eq(&vec![vec![],vec![120],vec![121],vec![122,122]],&d);
        assert_branch!(InterpNatural::Bytes,v.get_natural());
        assert_eq!(4,v.len());
    }

    fn range_f64_check(number: f64) -> bool {
        let v = Rc::new(InterpValueData::Numbers(vec![number]));
        let w = v.to_rc_indexes();
        if let Ok(ref w) = w {
            assert_eq!(w[0] as f64,number);
            assert_eq!(w[0],number as usize);
            let x = Rc::new(InterpValueData::Indexes(vec![w[0] as usize]));
            let w2 = x.to_rc_numbers();
            assert!(w2.is_ok());
            assert_eq!(w2.unwrap()[0],number);
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
        let mut v = Rc::new(InterpValueData::Numbers(vec![0.,1.,2.,3.]));
        let d = v.to_rc_indexes().expect("B");
        array_eq(&vec![0,1,2,3],&d);
        let mut e = Rc::get_mut(&mut v).unwrap().copy().to_numbers().expect("C");
        e[1] = -1.;
        let v = Rc::new(InterpValueData::Numbers(e));
        let d = v.to_rc_numbers().expect("B");
        array_eq(&vec![0.,-1.,2.,3.],&d);
    }
}
