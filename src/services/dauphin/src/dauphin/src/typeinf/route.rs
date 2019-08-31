use std::collections::HashMap;
use crate::model::Register;

#[derive(Clone,Debug)]
pub enum RouteExpr {
    Member(String),
    Square,
    Filter(Register)
}

#[derive(Clone,Debug)]
pub struct Route {
    route: HashMap<Register,(Register,Vec<RouteExpr>)>
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

    pub fn get(&self, reg: &Register) -> &(Register,Vec<RouteExpr>) {
        &self.route[reg]
    }
}