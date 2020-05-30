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
use regex::Regex;

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

    fn strip_extension(&self, name: &str) -> String {
        for extension in &EXTENSIONS {
            if name.ends_with(extension) {
                return name[0..name.len()-extension.len()].to_string();
            }
        }
        name.to_string()
    }

    fn get_module(&self, path: &PathBuf) -> Result<String,String> {
        if let Some(last) = path.iter().last() {
            let name = last.to_str().ok_or_else(|| format!("filename is bad unicode"))?;
            let name = self.strip_extension(name);
            let re = Regex::new(r"[^A-Za-z0-9]").unwrap();
            let name = re.replace_all(&name,"_");
            let re = Regex::new(r"^.*?:").unwrap();
            let name = re.replace_all(&name,"");
            Ok(name.to_string())
        } else {
            Ok("*anon*".to_string())
        }
    }
}

impl DocumentResolver for FileResolver {
    fn resolve(&self, name: &str, components: &str, _: &Resolver, new_resolver: &mut Resolver, prefix: &str) -> Result<Box<dyn CharSource>,String> {
        let path = if components.starts_with("/") {
            PathBuf::from(components)
        } else {
            self.add_components(components)
        };
        let module = self.get_module(&path)?;
        print!("components = {}\n",components);
        print!("path = {}\n",path.to_string_lossy());
        print!("name = {}\n",name);
        print!("module = {}\n",module);
        let mut dir = path.clone();
        dir.pop();
        let sub = FileResolver::new(dir);
        let data = read_to_string(path.clone()).map_err(|x| format!("{}: {}",path.to_str().unwrap_or(""),x))?;
        new_resolver.add(prefix,sub);
        Ok(Box::new(StringCharSource::new(name,&module,data)))
    }
}
