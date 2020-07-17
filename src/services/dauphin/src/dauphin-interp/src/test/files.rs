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

pub fn find_testdata() -> PathBuf {
    let mut dir = std::env::current_exe().expect("cannot get current exec path");
    while dir.pop() {
        let mut testdata = PathBuf::from(&dir);
        testdata.push("testdata");
        if testdata.exists() {
            return testdata;
        }
    }
    panic!("cannot find testdata directory");
}

pub fn load_testdata(tail: &[&str]) -> Result<String,String> {
    let mut path = find_testdata();
    for t in tail {
        path.push(t);
    }
    read_to_string(path).map_err(|x| x.to_string())
}