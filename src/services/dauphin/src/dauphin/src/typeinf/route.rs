use std::collections::HashMap;
use std::fmt;

use crate::model::Register;

#[derive(Clone,Debug)]
pub enum RouteExpr {
    Filter(Register),
    SeqFilter(Register,Register)
}

#[derive(Clone)]
pub struct Route {
    route: HashMap<Register,(Register,Vec<RouteExpr>)>
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut keys : Vec<Register> = self.route.keys().cloned().collect();
        keys.sort();
        for reg in &keys {
            let (ref origin,ref route) = self.route[reg];
            let route_str : Vec<String> = route.iter().map(|e| format!("{:?}",e)).collect();
            write!(f,"{:?} -> {:?} {}\n",reg,origin,route_str.join(" "))?;
        }
        write!(f,"\n")?;
        Ok(())
    }
}

impl Route {
    pub fn new() -> Route {
        Route {
            route: HashMap::new()
        }
    }

    pub fn set_empty(&mut self, reg: &Register, origin: &Register) {
        self.route.insert(reg.clone(),(origin.clone(),vec![]));
    }

    pub fn set_derive(&mut self, reg: &Register, origin: &Register, expr: &RouteExpr) {
        let mut new_route = self.route[origin].clone();
        new_route.1.push(expr.clone());
        self.route.insert(reg.clone(),new_route);
    }

    pub fn split_origin(&mut self, target: &Register, new_origin: &Register, source: &Register) {
        if let Some((_,expr)) = self.get(source) {
            let expr = expr.clone();
            self.route.insert(target.clone(),(new_origin.clone(),expr.clone()));
        }
    }

    pub fn get(&self, reg: &Register) -> Option<&(Register,Vec<RouteExpr>)> {
        self.route.get(reg)
    }

    pub fn remove(&mut self, reg: &Register) {
        self.route.remove(reg);
    }

    pub fn copy(&mut self, dst: &Register, src: &Register) {
        if let Some(route) = self.route.get(src) {
            let route = route.clone();
            self.route.insert(dst.clone(),route);
        }
    }

    pub fn quantify_member(&mut self, old_origin_reg: &Register, new_origin_reg: &Register) {
        let mut matching_origin = Vec::new();
        for (reg,(origin_reg,_)) in self.route.iter() {
            if origin_reg == old_origin_reg {
                matching_origin.push(reg.clone());
            }
        }
        for reg in &matching_origin {
            let expr = self.route[reg].1.clone();
            self.route.insert(reg.clone(),(new_origin_reg.clone(),expr));
            print!("gonna have to change {:?} to use {:?} ({:?} has been split)\n",reg,new_origin_reg,old_origin_reg);
        }
    }
}