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

use std::hash::{ Hash, Hasher };
use std::num::ParseFloatError;

#[derive(Clone,Debug)]
pub struct DFloat(f64,u64);

impl DFloat {
    pub fn new(input: f64) -> DFloat { // XXX get rid when compilerun is reformed and make this all safe
        DFloat(input,input.to_bits())
    }

    pub fn new_str(input: &str) -> Result<DFloat,ParseFloatError> {
        Ok(DFloat::new(input.parse()?))
    }

    pub fn new_usize(input: usize) -> DFloat {
        DFloat::new(input as f64)
    }

    pub fn as_f64(&self) -> f64 { self.0 }
}

impl PartialEq for DFloat {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for DFloat {}

impl Hash for DFloat {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.1.hash(hasher);
    }
}