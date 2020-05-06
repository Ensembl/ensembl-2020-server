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

use std::fs::read_to_string;
use std::path::PathBuf;
use super::core::{ DocumentResolver, Resolver };
use crate::lexer::{ CharSource, StringCharSource };

static EXTENSIONS : [&str;1] = [".dp"];

pub struct FileResolver {
    path: PathBuf
}

impl FileResolver {
    pub fn new(path: PathBuf) -> FileResolver {
        FileResolver  {
            path
        }
    }

    fn add_components(&self, components: &str) -> PathBuf {
        let mut path = self.path.clone();
        for component in components.split("/") {
            if component == ".." {
                path.pop();
            } else if component != "." {
                path.push(component);
            }
        }
        path
    }

    fn get_module(&self, path: &PathBuf) -> Result<String,String> {
        if let Some(last) = path.iter().last() {
            let last = last.to_str().ok_or_else(|| format!("filename is bad unicode"))?;
            for extension in &EXTENSIONS {
                if last.ends_with(extension) {
                    return Ok(last[0..last.len()-extension.len()].to_string());
                }
            }
            Ok(last.to_string())
        } else {
            Ok("*anon*".to_string())
        }
    }
}

impl DocumentResolver for FileResolver {
    fn resolve(&self, components: &str, _: &Resolver, new_resolver: &mut Resolver, prefix: &str) -> Result<Box<dyn CharSource>,String> {
        let path = self.add_components(components);
        let module = self.get_module(&path)?;
        let mut dir = path.clone();
        dir.pop();
        let sub = FileResolver::new(dir);
        let data = read_to_string(path.clone()).map_err(|x| format!("{}: {}",path.to_str().unwrap_or(""),x))?;
        new_resolver.add(prefix,sub);
        Ok(Box::new(StringCharSource::new(components,&module,data)))
    }
}
