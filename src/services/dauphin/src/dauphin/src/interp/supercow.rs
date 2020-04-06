use std::cell::Cell;
use std::rc::Rc;

pub struct SuperCow<T> {
    copy: Box<dyn Fn(&T) -> T>,
    set: Option<Rc<T>>,
    get: Option<Rc<T>>
}

fn cell_copy<T>(data: &Cell<Option<Rc<T>>>) -> Option<Rc<T>> {
    let v = data.take();
    let out = v.clone();
    data.set(v);
    out
}

impl<T> SuperCow<T> {
    pub fn new<F>(data: T, copy: F) -> SuperCow<T> where F: Fn(&T) -> T + Clone + 'static {
        SuperCow {
            copy: Box::new(copy),
            set: None,
            get: Some(Rc::new(data))
        }
    }

    pub fn copy(&mut self, other: &SuperCow<T>) -> Result<(),String> {
        if let Some(ref src) = other.get {
            self.set = Some(src.clone());
            Ok(())
        } else {
            Err(format!("Attempt to copy modifying register"))
        }
    }

    pub fn get_shared(&self) -> Result<Rc<T>,String> {
        Ok(self.get.clone().ok_or_else(|| format!("Attempt to read modifying register"))?)
    }

    pub fn get_exclusive(&mut self) -> Result<T,String> {
        let mut get = self.get.take().clone().ok_or_else(|| format!("Attempt double-modify modify register"))?;
        if Rc::strong_count(&get) + Rc::weak_count(&get) > 1 {
            get = Rc::new((self.copy)(&get));
        }
        Ok(Rc::try_unwrap(get).map_err(|_| format!("unwrap failed"))?)
    }

    pub fn set(&mut self, value: T) {
        self.set = Some(Rc::new(value));
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
