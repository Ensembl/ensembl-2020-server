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

use std::path::Path;
use super::core::DocumentResolver;
use crate::lexer::CharSource;

use super::core::Resolver;
use crate::lexer::StringCharSource;
use super::common::{ DataResolver, PreambleResolver };
use super::file::FileResolver;
use super::search::SearchResolver;
use crate::test::files::{ find_testdata, load_testdata };

pub struct TestResolver {}

impl TestResolver {
    pub fn new() -> TestResolver {
        TestResolver {}
    }
}

impl DocumentResolver for TestResolver {
    fn resolve(&self, path: &str, _: &Resolver, _: &mut Resolver, _: &str) -> Result<Box<dyn CharSource>,String> {
        let paths : Vec<&str> = path.split("/").collect();
        let name = format!("test:{}",path);
        match load_testdata(&paths) {
            Ok(data) => Ok(Box::new(StringCharSource::new(&name,"test",data))),
            Err(err) => Err(format!("Loading \"{}\": {}",path,err))
        }
    }
}

#[cfg(test)]
pub fn test_resolver() -> Resolver {
    let root_dir = find_testdata();
    let mut out = Resolver::new();
    let std_path = root_dir.clone();
    let std_path = Path::new(&std_path)
        .parent().unwrap_or(&std_path)
        .join("src").join("commands").join("std");
    let bt_path = root_dir.clone();
    let bt_path = Path::new(&bt_path)
        .parent().unwrap_or(&bt_path)
        .join("src").join("commands").join("buildtime");
    print!("root path {}\n",root_dir.display());
    print!("std path {}\n",std_path.display());
    out.add("preamble",PreambleResolver::new());
    out.add("test",TestResolver::new());
    out.add("data",DataResolver::new());
    out.add("file",FileResolver::new(root_dir));
    out.add("search",SearchResolver::new(&vec![
        format!("file:{}/*.dp",std_path.to_string_lossy()),
        format!("file:{}/*.dp",bt_path.to_string_lossy()),
    ]));
    out
}
