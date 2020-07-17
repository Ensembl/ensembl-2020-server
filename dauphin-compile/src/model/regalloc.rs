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

use std::cell::RefCell;
use std::rc::Rc;
use dauphin_interp::runtime::{ Register };

#[derive(Debug)]
struct RegisterAllocatorImpl {
    index: usize
}

impl RegisterAllocatorImpl {
    fn new(start: usize) -> RegisterAllocatorImpl {
        RegisterAllocatorImpl {
            index: start
        }
    }

    fn allocate(&mut self) -> Register {
        self.index += 1;
        Register(self.index)
    }
}

#[derive(Clone,Debug)]
pub struct RegisterAllocator(Rc<RefCell<RegisterAllocatorImpl>>);

impl RegisterAllocator {
    pub fn new(start: usize) -> RegisterAllocator {
        RegisterAllocator(Rc::new(RefCell::new(RegisterAllocatorImpl::new(start))))
    }

    pub fn allocate(&self) -> Register {
        self.0.borrow_mut().allocate().clone()
    }
}
