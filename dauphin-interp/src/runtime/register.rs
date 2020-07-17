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

use std::fmt;
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

#[cfg(test)]
mod test {
    use crate::runtime::{ Register, InterpValue, RegisterFile };

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
