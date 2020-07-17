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
use crate::runtime::{ InterpValue };
use crate::types::{ arbitrate_type };

fn update_poly<T>(dst: &mut Vec<T>, src: &Vec<T>, filter: &[usize]) where T: Clone {
    let mut target = vec![];
    while target.len() < filter.len() {
        target.append(&mut src.to_vec());
    }
    let mut value_it = target.drain(..);
    for index in filter.iter() {
        dst[*index] = value_it.next().unwrap();
    }
}

pub fn vector_update_poly(dst: InterpValue, src: &Rc<InterpValue>, filter_val: &[usize]) -> Result<InterpValue,String> {
    if let Some(natural) = arbitrate_type(&dst,src,true) {
        Ok(polymorphic!(dst,[src],natural,(|d,s| {
            update_poly(d,s,filter_val)
        })))
    } else {
        Ok(dst)
    }
}

pub fn append_data(dst: InterpValue, src: &Rc<InterpValue>, copies: usize) -> Result<(InterpValue,usize),String> {
    let offset = src.len();
    if let Some(natural) = arbitrate_type(&dst,src,false) {
        Ok((polymorphic!(dst,[src],natural,(|d: &mut Vec<_>, s: &[_]| {
            for _ in 0..copies {
                d.append(&mut s.to_vec());
            }
        })),offset))
    } else {
        Ok((dst,offset))
    }
}
