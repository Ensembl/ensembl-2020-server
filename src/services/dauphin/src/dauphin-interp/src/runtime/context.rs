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

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use crate::runtime::RegisterFile;

pub trait PayloadFactory {
    fn make_payload(&self) -> Box<dyn Any>;
}

pub struct InterpContext {
    registers: RegisterFile,
    payloads: HashMap<(String,String),Box<dyn Any>>,
    filename: String,
    line_number: u32,
    pause: bool
}

impl InterpContext {
    pub fn new(payloads: &HashMap<(String,String),Rc<Box<dyn PayloadFactory>>>) -> InterpContext {
        InterpContext {
            registers: RegisterFile::new(),
            payloads: payloads.iter().map(|(k,v)| (k.clone(),v.make_payload())).collect(),
            filename: "**anon**".to_string(),
            line_number: 0,
            pause: false
        }
    }

    pub fn do_pause(&mut self) { self.pause = true; }
    pub fn test_pause(&mut self) -> bool {
        let out = self.pause;
        self.pause = false;
        out
    }
    pub fn registers(&self) -> &RegisterFile { &self.registers }
    pub fn registers_mut(&mut self) -> &mut RegisterFile { &mut self.registers }
    pub fn payload(&mut self, set: &str, name: &str) -> Result<&mut Box<dyn Any>,String> {
        self.payloads.get_mut(&(set.to_string(),name.to_string())).ok_or_else(|| format!("missing payload {}",name))
    }

    pub fn set_line_number(&mut self, filename: &str, line_number: u32) {
        self.filename = filename.to_string();
        self.line_number = line_number;
    }

    pub fn get_line_number(&self) -> (&str,u32) {
        (&self.filename,self.line_number)
    }
}
