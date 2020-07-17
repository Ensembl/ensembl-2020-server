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

use std::collections::HashMap;
use std::path::PathBuf;
use crate::cli::Config;
use crate::lexer::StringCharSource;
use super::core::{ DocumentResolver, Resolver, ResolverQuery, ResolverResult };
use super::preamble::PREAMBLE;
use super::file::FileResolver;
use super::search::SearchResolver;
use crate::command::CompilerLink;

pub struct DataResolver {}

impl DataResolver {
    pub fn new() -> DataResolver {
        DataResolver {}
    }
}

impl DocumentResolver for DataResolver {
    fn resolve(&self, query: &ResolverQuery) -> Result<ResolverResult,String> {
        let path = query.current_suffix();
        Ok(query.new_result(StringCharSource::new(query.original_name(),"data",path.to_string())))
    }
}

pub struct HashMapResolver(HashMap<String,String>);

impl HashMapResolver {
    pub fn new(values: &HashMap<String,String>) -> HashMapResolver {
        HashMapResolver(values.clone())
    }
}

impl DocumentResolver for HashMapResolver {
    fn resolve(&self, query: &ResolverQuery) -> Result<ResolverResult,String> {
        let key = query.current_suffix();
        if let Some(value) = self.0.get(key) {
            Ok(query.new_result(StringCharSource::new(query.original_name(),key,value.to_string())))
        } else {
            Err(format!("No such library header 'lib:{}'",key))
        }
    }
}

pub struct PreambleResolver();

impl PreambleResolver {
    pub fn new() -> PreambleResolver {
        PreambleResolver()
    }
}

impl DocumentResolver for PreambleResolver {
    fn resolve(&self, query: &ResolverQuery) -> Result<ResolverResult,String> {
        Ok(query.new_result(StringCharSource::new("preamble","preamble",PREAMBLE.to_string())))
    }
}

fn root_dir(config: &Config) -> Result<PathBuf,String> {
    if config.isset_root_dir() {
        Ok(PathBuf::from(config.get_root_dir()))
    } else {
        std::env::current_dir().map_err(|x| x.to_string())
    }
}

fn calculate_search_path(config: &Config) -> Vec<String> {
    let mut out = vec![];
    for path in config.get_file_search_path() {
        out.push(format!("root:{}",path))
    }
    out
}

pub fn common_resolver(config: &Config, clink: &CompilerLink) -> Result<Resolver,String> {
    let root_dir = root_dir(config)?;
    let mut out = Resolver::new(config);
    out.add("preamble",PreambleResolver::new());
    out.add("data",DataResolver::new());
    out.add("file",FileResolver::new(&root_dir));
    out.add("root",FileResolver::new(&root_dir));
    out.add("search",SearchResolver::new(&calculate_search_path(config)));
    out.add("lib",HashMapResolver::new(clink.get_headers()));
    Ok(out)
}
