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

use regex::Regex;

pub fn fix_filename(s: &str) -> String {
    let invalid = Regex::new("/").expect("bad regex in fix_filename");
    invalid.replace_all(s,"-").to_string()
}

pub fn fix_incoming_filename(name: &str) -> String {
    let re = Regex::new(r"[^A-Za-z0-9]+").unwrap();
    let name = re.replace_all(&name,"_");
    let re = Regex::new(r".*/").unwrap();
    let name = re.replace_all(&name,"");
    let re = Regex::new(r"\.dp").unwrap();
    let name = re.replace_all(&name,"");
    name.to_string()
}