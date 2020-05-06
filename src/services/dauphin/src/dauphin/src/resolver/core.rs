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
use std::collections::HashMap;

use crate::lexer::CharSource;

pub trait DocumentResolver {
    fn resolve(&self, path: &str, current_resolver: &Resolver, new_resolver: &mut Resolver, prefix: &str) -> Result<Box<dyn CharSource>,String>;
}

#[derive(Clone)]
pub struct Resolver {
    document_resolvers: HashMap<String,Rc<dyn DocumentResolver>>
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {
            document_resolvers: HashMap::new()
        }
    }

    pub fn add<T>(&mut self, prefix: &str, document_resolver: T) where T: DocumentResolver + 'static {
        self.document_resolvers.insert(prefix.to_string(),Rc::new(document_resolver));
    }
    
    pub fn document_resolve(&self, new_resolver: &mut Resolver, path: &str) -> Result<Box<dyn CharSource>,String> {
        let (our_prefix,our_suffix) = if let Some(colon) = path.find(':') {
            (&path[0..colon],&path[colon+1..])
        } else {
            ("",path)
        };
        if let Some(document_resolver) = self.document_resolvers.get(our_prefix) {
            document_resolver.resolve(our_suffix,&self,new_resolver,our_prefix)
        } else {
            Err(format!("protocol {} not supported",our_prefix))
        }
    }

    pub fn resolve(&self, path: &str) -> Result<(Box<dyn CharSource>,Resolver),String> {
        let mut sub = self.clone();
        let source = self.document_resolve(&mut sub,path)?;
        Ok((source,sub))
    }
}
