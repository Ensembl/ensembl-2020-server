use std::fmt;
use crate::typeinf::BaseType;
use serde_cbor::Value as CborValue;
use crate::model::{ cbor_int, cbor_array };

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct VectorRegisters {
    start: usize,
    depth: usize,
    base: BaseType    
}

impl VectorRegisters {
    pub(super) fn new(depth: usize, base: BaseType) -> VectorRegisters {
        VectorRegisters {
            depth, base,
            start: 0
        }
    }

    pub fn deserialize(cbor: &CborValue) -> Result<VectorRegisters,String> {
        let data = cbor_array(cbor,2,false)?;
        Ok(VectorRegisters::new(cbor_int(&data[0],None)? as usize,BaseType::deserialize(&data[1])?))
    }

    pub fn serialize(&self) -> Result<CborValue,String> {
        Ok(CborValue::Array(vec![CborValue::Integer(self.depth as i128),self.base.serialize()?]))
    }

    pub(super) fn add_start(&mut self, start: usize) {
        self.start += start;
    }

    pub fn depth(&self) -> usize { self.depth }
    pub fn data_pos(&self) -> usize { self.start }

    pub fn lower_pos(&self, level: usize) -> usize {
        if level > 0 { self.offset_pos(level-1).unwrap() } else { self.data_pos() }
    }

    pub fn offset_pos(&self, level: usize) -> Result<usize,String> {
        if self.depth > level {
            Ok(self.start+level*2+1)
        } else {
            Err(format!("bad level {}. depth is {}",level,self.depth))
        }
    }

    pub fn length_pos(&self, level: usize) -> Result<usize,String> {
        if self.depth > level {
            Ok(self.start+level*2+2)
        } else {
            Err(format!("bad level {}. depth is {}",level,self.depth))
        }
    }

    pub fn register_count(&self) -> usize { self.depth*2+1 }
}

impl fmt::Display for VectorRegisters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(write!(f,"<{}:{:?}>",self.depth,self.base)?)
    }
}
