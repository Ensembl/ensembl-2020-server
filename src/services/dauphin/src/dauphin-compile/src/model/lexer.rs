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
use std::rc::Rc;

#[derive(Debug,PartialEq,Eq,Hash,Clone)]
pub struct FileContentsHandle {
    contents: String
}

impl FileContentsHandle {
    pub fn new(contents: &str) -> FileContentsHandle {
        FileContentsHandle { contents: contents.to_string() }
    }

    pub(crate) fn get(&self) -> String { self.contents.to_string() }
}

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub struct LexerPosition {
    handle: Option<Rc<FileContentsHandle>>,
    filename: String,
    line: u32,
    col: u32
}

impl LexerPosition {
    pub fn new(filename: &str, line: u32, col: u32, handle: Option<&Rc<FileContentsHandle>>) -> LexerPosition {
        LexerPosition {
            handle: handle.cloned(),
            filename: filename.to_string(), line, col
        }
    }

    pub fn filename(&self) -> &str { &self.filename }
    pub fn line(&self) -> u32 { self.line }
    #[allow(unused)]
    pub fn col(&self) -> u32 { self.col }
    pub fn contents(&self) -> Option<String> { self.handle.as_ref().map(|x| x.get()) }
}

impl fmt::Display for LexerPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}:{}:{}",self.filename,self.line,self.col)
    }
}
