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

pub struct SuperCow<T> {
    copy: Box<dyn Fn(&T) -> T>,
    set: Option<Rc<T>>,
    get: Option<Rc<T>>
}

impl<T> SuperCow<T> {
    pub fn new<F>(data: T, copy: F) -> SuperCow<T> where F: Fn(&T) -> T + 'static {
        SuperCow {
            copy: Box::new(copy),
            set: None,
            get: Some(Rc::new(data))
        }
    }

    pub fn copy(&mut self, other: &SuperCow<T>) -> Result<(),String> {
        self.set = Some(other.get.as_ref().ok_or_else(|| format!("Attempt to copy with exclusive value"))?.clone());
        Ok(())
    }

    pub fn get_shared(&self) -> Result<Rc<T>,String> {
        Ok(self.get.clone().ok_or_else(|| format!("Attempt to share exclusive value"))?)
    }

    pub fn get_exclusive(&mut self) -> Result<T,String> {
        let value = if let Some(value) = self.set.take() {
            value.clone()
        } else if let Some(value) = self.get.take() {
            value.clone()
        } else {
            return Err(format!("Attempt to double spend exclusive value"));
        };
        Ok(Rc::try_unwrap(value).unwrap_or_else(|rc| (self.copy)(&rc)))
    }

    pub fn set(&mut self, value: T) {
        self.set_rc(Rc::new(value));
    }

    pub fn set_rc(&mut self, value: Rc<T>) {
        self.set = Some(value);
    }
}

pub trait SuperCowCommit {
    fn commit(&mut self);
}

impl<T> SuperCowCommit for SuperCow<T> {
    fn commit(&mut self) {
        if let Some(set) = self.set.take() {
            self.get = Some(set);
        }
    }
}
