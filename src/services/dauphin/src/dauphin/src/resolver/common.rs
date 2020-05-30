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

use crate::cli::Config;
use crate::lexer::{ CharSource, StringCharSource };
use super::core::{ DocumentResolver, Resolver };
use super::preamble::PREAMBLE;
use super::file::FileResolver;
use super::search::SearchResolver;

pub struct DataResolver {}

impl DataResolver {
    pub fn new() -> DataResolver {
        DataResolver {}
    }
}

impl DocumentResolver for DataResolver {
    fn resolve(&self, _name: &str, path: &str, _: &Resolver, _: &mut Resolver, _: &str) -> Result<Box<dyn CharSource>,String> {
        Ok(Box::new(StringCharSource::new(&format!("data:{}",path),"data",path.to_string())))
    }
}

pub struct PreambleResolver();

impl PreambleResolver {
    pub fn new() -> PreambleResolver {
        PreambleResolver()
    }
}

impl DocumentResolver for PreambleResolver {
    fn resolve(&self, name: &str, path: &str, _: &Resolver, _: &mut Resolver, _: &str) -> Result<Box<dyn CharSource>,String> {
        if path == "" {
            Ok(Box::new(StringCharSource::new("preamble","preamble",PREAMBLE.to_string())))
        } else {
            Err(format!("Nonsense in preamble path"))
        }
    }
}

pub fn common_resolver(config: &Config) -> Result<Resolver,String> {
    let root_dir = std::env::current_dir().map_err(|x| x.to_string())?;
    let mut out = Resolver::new();
    out.add("preamble",PreambleResolver::new());
    out.add("data",DataResolver::new());
    out.add("file",FileResolver::new(root_dir));
    out.add("search",SearchResolver::new(config.get_search_path()));
    Ok(out)
}
