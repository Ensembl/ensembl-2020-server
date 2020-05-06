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

use super::core::DocumentResolver;
use crate::lexer::CharSource;

use super::core::Resolver;

pub struct SearchResolver {
    templates: Vec<String>
}

impl SearchResolver {
    pub fn new(templates: &[String]) -> SearchResolver {
        SearchResolver {
            templates: templates.to_vec()
        }
    }
}

impl DocumentResolver for SearchResolver {
    fn resolve(&self, path: &str, resolver: &Resolver, new_resolver: &mut Resolver, _: &str) -> Result<Box<dyn CharSource>,String> {
        let mut errors = vec![];
        for template in &self.templates {
            let new_path = template.replace("*",path);
            match resolver.document_resolve(new_resolver,&new_path) {
                Ok(out) => { return Ok(out); },
                Err(err) => { errors.push(err); }
            }
        }
        Err(format!("not found in search path: {}",errors.join(", ")))
    }
}
