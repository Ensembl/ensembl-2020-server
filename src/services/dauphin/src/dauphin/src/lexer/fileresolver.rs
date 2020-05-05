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

#[cfg(test)]
use crate::test::files::load_testdata;

use super::charsource::{ CharSource, StringCharSource };
use super::preamble::PREAMBLE;

pub struct FileResolver {

}

// TODO use in proper fs-based compiler
fn base_path(filename: &str) -> String {
    if let Some(last) = filename.split("/").last() {
        if last.ends_with(".dp") {
            last[..last.len()-3].to_string()
        } else {
            last.to_string()
        }
    } else {
        "".to_string()
    }

}

impl FileResolver {
    pub fn new() -> FileResolver {
        FileResolver {}
    }

    #[cfg(test)]
    fn test_path(&self, path: &str) -> Result<Box<dyn CharSource>,String> {
        let paths : Vec<&str> = path.split("/").collect();
        let name = format!("test:{}",path);
        match load_testdata(&paths) {
            Ok(data) => Ok(Box::new(StringCharSource::new(&name,"test",data))),
            Err(err) => Err(format!("Loading \"{}\": {}",path,err))
        }
    }

    #[cfg(not(test))]
    fn test_path<'a>(&self, _path: &'a str) -> Result<Box<dyn CharSource>,String> {
        Err("no test files except when running tests".to_string())
    }

    pub fn resolve(&self, path: &str) -> Result<Box<dyn CharSource>,String> {
        if path.starts_with("data:") {
            Ok(Box::new(StringCharSource::new(path,"data",path[5..].to_string())))
        } else if path.starts_with("test:") {
            self.test_path(&path[5..])
        } else if path.starts_with("preamble:") {
            Ok(Box::new(StringCharSource::new(path,"preamble",PREAMBLE.to_string())))
        } else {
            Err("protocol not supported".to_string())
        }
    }
}