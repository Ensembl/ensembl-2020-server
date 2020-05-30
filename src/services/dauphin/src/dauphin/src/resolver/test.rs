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

use std::env::set_current_dir;
use crate::interp::find_testdata;
use super::core::Resolver;
use crate::interp::xxx_test_config;
use crate::resolver::common_resolver;

#[cfg(test)]
pub fn test_resolver() -> Result<Resolver,String> {
    let path = find_testdata();
    let path = path.parent().unwrap();
    set_current_dir(path).expect("A");
    let config = xxx_test_config();
    let out = common_resolver(&config)?;
    Ok(out)
}
