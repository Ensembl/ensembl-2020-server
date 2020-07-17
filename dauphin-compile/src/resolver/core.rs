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

use std::rc::Rc;
use crate::cli::Config;
use crate::resolver::ResolveFile;
use std::collections::HashMap;
use crate::lexer::CharSource;

pub(super) fn prefix_suffix(path: &str) -> (&str,&str) {
    if let Some(colon) = path.find(':') {
        print!("prefix_suffix({}) = ({},{})\n",path,&path[0..colon],&path[colon+1..]);
        (&path[0..colon],&path[colon+1..])
    } else {
        print!("prefix_suffix({}) = ({},{})\n",path,"",path);
        ("",path)
    }
}

pub struct ResolverQuery<'a> {
    current_prefix: String,
    current_suffix: String,
    original_name: String,
    current_resolver: &'a Resolver
}

impl<'a> ResolverQuery<'a> {
    fn new_internal(resolver: &'a Resolver, original: &str, current: &str) -> ResolverQuery<'a> {
        let (prefix,suffix) = prefix_suffix(current);
        ResolverQuery {
            current_resolver: resolver,
            current_prefix: prefix.to_string(),
            current_suffix: suffix.to_string(),
            original_name: original.to_string()
        }
    }

    pub fn new(resolver: &'a Resolver, name: &str) -> ResolverQuery<'a> {
        ResolverQuery::new_internal(resolver,name,name)
    }

    pub fn new_subquery(&self, name: &str) -> ResolverQuery<'a> {
        ResolverQuery::new_internal(self.current_resolver,&self.original_name,name)
    }

    pub fn original_name(&self) -> &str { &self.original_name }
    pub fn current_prefix(&self) -> &str { &self.current_prefix }
    pub fn current_suffix(&self) -> &str { &self.current_suffix }
    pub fn resolver(&self) -> &Resolver { &self.current_resolver }

    pub fn new_result<S>(&self, source: S) -> ResolverResult where S: CharSource + 'static {
        ResolverResult::new(source,self.current_resolver.clone())
    }
}

pub struct ResolverResult {
    source: Box<dyn CharSource>,
    resolver: Resolver
}

impl ResolverResult {
    fn new<S>(source: S, resolver: Resolver) -> ResolverResult where S: CharSource + 'static {
        ResolverResult {
            source: Box::new(source),
            resolver
        }
    }

    pub fn resolver(&mut self) -> &mut Resolver { &mut self.resolver }
}

pub trait DocumentResolver {
    fn resolve(&self, query: &ResolverQuery) -> Result<ResolverResult,String>;
}

#[derive(Clone)]
pub struct Resolver {
    config: Config,
    document_resolvers: HashMap<String,Rc<dyn DocumentResolver>>
}

impl Resolver {
    pub fn new(config: &Config) -> Resolver {
        Resolver {
            config: config.clone(),
            document_resolvers: HashMap::new()
        }
    }

    pub fn config(&self) -> &Config { &self.config }

    pub fn add<T>(&mut self, prefix: &str, document_resolver: T) where T: DocumentResolver + 'static {
        self.document_resolvers.insert(prefix.to_string(),Rc::new(document_resolver));
    }
    
    pub fn document_resolve(&self, query: &ResolverQuery) -> Result<ResolverResult,String> {
        let our_prefix = query.current_prefix();
        if let Some(document_resolver) = self.document_resolvers.get(our_prefix) {
            document_resolver.resolve(&query)
        } else {
            Err(format!("protocol {} not supported",our_prefix))
        }
    }

    pub fn resolve(&self, path: &str) -> Result<(Box<dyn CharSource>,Resolver),String> {
        let query = ResolverQuery::new(&self,path);
        let source = self.document_resolve(&query)?;
        Ok((source.source,source.resolver))
    }
}

impl ResolveFile for Resolver {
    fn resolve(&self, path: &str) -> Result<String,String> {
        Ok(self.resolve(path)?.0.to_string())
    }
}