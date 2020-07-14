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
use crate::interp::{ InterpValue };
use crate::interp::{ PayloadFactory };

#[derive(Debug)]
pub enum StreamContents {
    String(String),
    Data(InterpValue),
}

#[derive(Debug)]
pub struct Stream {
    contents: Vec<StreamContents>
}

impl Stream {
    pub fn new() -> Stream {
        Stream {
            contents: Vec::new()
        }
    }

    pub fn add(&mut self, contents: StreamContents) {
        self.contents.push(contents);
    }

    pub fn take(&mut self) -> Vec<StreamContents> {
        self.contents.drain(..).collect()
    }
}

pub struct StreamFactory {}

impl StreamFactory {
    pub fn new() -> StreamFactory {
        StreamFactory {}
    }
}

impl PayloadFactory for StreamFactory {
    fn make_payload(&self) -> Box<dyn Any> {
        Box::new(Stream::new())
    }
}