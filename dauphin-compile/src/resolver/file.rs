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
use super::core::{ DocumentResolver, ResolverQuery, ResolverResult };
use crate::lexer::StringCharSource;
use regex::Regex;

static EXTENSIONS : [&str;1] = [".dp"];

pub struct FileResolver {
    path: PathBuf
}

impl FileResolver {
    pub fn new(path: &PathBuf) -> FileResolver {
        FileResolver  {
            path: path.clone()
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
            let re = Regex::new(r"[^A-Za-z0-9]+").unwrap();
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
    fn resolve(&self, query: &ResolverQuery) -> Result<ResolverResult,String> {
        let name = query.current_suffix();
        let path = if name.starts_with("/") {
            PathBuf::from(name)
        } else {
            let out = self.add_components(name);
            if query.resolver().config().get_verbose() > 1 {
                print!("converting {} to absolute path using {} as root with result {}\n",
                    name,self.path.to_string_lossy(),out.to_string_lossy());
            }
            out
        };
        let module = self.get_module(&path)?;
        if query.resolver().config().get_verbose() > 1 {
            print!("found {}:{} at {}. Using module name {}\n",
                query.current_prefix(),
                query.current_suffix(),
                path.to_string_lossy(),
                module);
                
        }
        let mut dir = path.clone();
        dir.pop();
        let sub = FileResolver::new(&dir);
        let data = read_to_string(path.clone()).map_err(|x| format!("{}: {}",path.to_str().unwrap_or(""),x))?;
        let mut result = query.new_result(StringCharSource::new(query.original_name(),&module,data));
        result.resolver().add("file",sub);
        Ok(result)
    }
}
