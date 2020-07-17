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


#[cfg(test)]
mod test {
    use std::sync::Mutex;
    use super::*;

    lazy_static! {
        static ref THING_NUMBER: Mutex<usize> = Mutex::new(0);
    }

    #[derive(PartialEq,Debug)]
    struct Thing(usize,usize);

    fn thing(key: usize) -> Thing { let mut x = THING_NUMBER.lock().unwrap(); *x += 1; Thing(*x,key) }

    #[test]
    fn supercow_smoke() {
        let t1 = thing(0);
        let t1r = Thing(t1.0,t1.1);
        let mut s = SuperCow::new(t1, |x| thing(x.1) );
        let t1a = s.get_shared().expect("A");
        assert_eq!(t1r.1,t1a.1);
        assert_eq!(t1r.0,t1a.0);
        let t1b = s.get_exclusive().expect("B");
        assert_eq!(t1r.1,t1b.1);
        assert_ne!(t1r.0,t1b.0);
        let t1s = Thing(t1b.0,t1b.1);
        s.set(t1b);
        s.commit();
        drop(t1a);
        let t1b = s.get_exclusive().expect("C");
        assert_eq!(t1s.1,t1b.1);
        assert_eq!(t1s.0,t1b.0);
        s.get_shared().expect_err("D");
        s.get_exclusive().expect_err("E");
        s.set(t1b);
        s.get_shared().expect_err("F");
        s.commit();
        let t1c = s.get_shared().expect("G");
        assert_eq!(t1s.1,t1c.1);
        assert_eq!(t1s.0,t1c.0);
    }

    #[test]
    fn supercow_copied_on_shared() {
        let t1 = thing(0);
        let t1r = Thing(t1.0,t1.1);
        let mut s = SuperCow::new(t1, |x| thing(x.1) );
        let t1a = s.get_shared().expect("A");
        assert_eq!(t1r.1,t1a.1);
        assert_eq!(t1r.0,t1a.0);
        let t1b = s.get_exclusive().expect("B");
        assert_eq!(t1r.1,t1b.1);
        assert_ne!(t1r.0,t1b.0);
    }

    #[test]
    fn supercopy_no_copy_on_not_shared() {
        let t1 = thing(0);
        let t1r = Thing(t1.0,t1.1);
        let mut s = SuperCow::new(t1, |x| thing(x.1) );
        let t1b = s.get_exclusive().expect("B");
        assert_eq!(t1r.1,t1b.1);
        assert_eq!(t1r.0,t1b.0);
    }

    #[test]
    fn supercow_copy() {
        let t1 = thing(1);
        let t10 = t1.0;
        let t2 = thing(2);
        let mut s1 = SuperCow::new(t1, |x| thing(x.1) );
        let mut s2 = SuperCow::new(t2, |x| thing(x.1) );
        assert_eq!(1,s1.get_shared().expect("A").1);
        assert_eq!(2,s2.get_shared().expect("B").1);
        s2.copy(&s1).expect("G");
        assert_eq!(1,s1.get_shared().expect("C").1);
        assert_eq!(2,s2.get_shared().expect("D").1);
        s2.commit();
        assert_eq!(1,s1.get_shared().expect("E").1);
        assert_eq!(1,s2.get_shared().expect("F").1);
        let mut t1a = s1.get_exclusive().expect("H");
        assert_ne!(t1a.0,t10);
        t1a.1 = 3;
        assert_eq!(1,s2.get_shared().expect("I").1);
        s1.get_shared().expect_err("J");
        s1.set(t1a);
        s1.commit();
        assert_eq!(3,s1.get_shared().expect("K").1);
        assert_eq!(1,s2.get_shared().expect("L").1);
    }

    #[test]
    fn test_copy_on_exc_fail() {
        let t1 = thing(1);
        let t2 = thing(2);
        let mut s1 = SuperCow::new(t1, |x| thing(x.1) );
        let mut s2 = SuperCow::new(t2, |x| thing(x.1) );
        let x = s1.get_exclusive().expect("A");
        s2.copy(&s1).expect_err("B");
        s1.set(x);
    }

    #[test]
    fn test_double_copy() {
        let t1 = thing(1);
        let t2 = thing(2);
        let t3 = thing(3);
        let s1 = SuperCow::new(t1, |x| thing(x.1) );
        let mut s2 = SuperCow::new(t2, |x| thing(x.1) );
        let s3 = SuperCow::new(t3, |x| thing(x.1) );
        s2.copy(&s1).expect("A");
        s2.copy(&s3).expect("B");
        s2.commit();
        assert_eq!(3,s2.get_shared().expect("C").1);
    }

    #[test]
    fn test_shared_exc() {
        let t1 = thing(1);
        let mut s1 = SuperCow::new(t1, |x| thing(x.1) );
        let _x = s1.get_exclusive().expect("A");
        s1.get_shared().expect_err("B");
    }
}
