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
use std::hash::{ Hash, Hasher };
use crate::util::cbor::{ cbor_array, cbor_string, cbor_int };
use serde_cbor::Value as CborValue;

#[derive(Clone,Eq,Debug,PartialOrd,Ord)]
pub struct CommandSetId {
    name: String,
    version: (u32,u32),
    trace: u64
}

impl Hash for CommandSetId {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher);
        self.version.hash(hasher);
    }
}

impl PartialEq for CommandSetId {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

impl fmt::Display for CommandSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}/{}.{}",self.name,self.version.0,self.version.1)
    }
}

impl CommandSetId {
    pub fn new(name: &str, version: (u32,u32), trace: u64) -> CommandSetId {
        CommandSetId { name: name.to_string(), version, trace }
    }

    pub fn trace(&self) -> u64 { self.trace }

    pub fn name(&self) -> &str { &self.name }
    pub fn version(&self) -> (u32,u32) { self.version }

    pub fn serialize(&self) -> CborValue {
        CborValue::Array(vec![
            CborValue::Text(self.name.to_string()),
            CborValue::Integer(self.version.0 as i128), CborValue::Integer(self.version.1 as i128),
            CborValue::Integer(self.trace as i128)
        ])
    }

    pub fn deserialize(cbor: &CborValue) -> Result<CommandSetId,String> {
        let data = cbor_array(cbor,4,false)?;
        Ok(CommandSetId {
            name: cbor_string(&data[0])?,
            version: (cbor_int(&data[1],None)? as u32,cbor_int(&data[2],None)? as u32),
            trace: cbor_int(&data[3],None)? as u64
        })
    }
}
#[cfg(test)]
mod test {
    use crate::command::{ CommandSetId };
    use crate::test::cbor_cmp;

    #[test]
    fn test_commandsetid_smoke() {
        let csi = CommandSetId::new("test",(1,2),0xDEADBEEFCABBA9E5);
        let csi2 = CommandSetId::deserialize(&csi.serialize()).expect("a");
        assert_eq!(0xDEADBEEFCABBA9E5,csi2.trace());
        assert_eq!("test",csi2.name());
        assert_eq!((1,2),csi2.version());
        cbor_cmp(&csi2.serialize(),"commandsetid.out");
        assert_eq!(csi,csi2);
        assert_eq!("test/1.2",csi.to_string());
    }
}
