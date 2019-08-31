use std::collections::HashMap;
use crate::codegen::Register2;

#[derive(Clone,Debug)]
pub enum RouteExpr {
    Member(String),
    Square,
    Filter(Register2)
}

#[derive(Clone,Debug)]
pub struct Route {
    route: HashMap<Register2,(Register2,Vec<RouteExpr>)>
}

impl Route {
    pub fn new() -> Route {
        Route {
            route: HashMap::new()
        }
    }

    pub fn set_empty(&mut self, reg: &Register2, origin: &Register2) {
        self.route.insert(reg.clone(),(origin.clone(),vec![]));
    }

    pub fn set_derive(&mut self, reg: &Register2, origin: &Register2, expr: &RouteExpr) {
        let mut new_route = self.route[origin].clone();
        new_route.1.push(expr.clone());
        self.route.insert(reg.clone(),new_route);
    }

    pub fn get(&self, reg: &Register2) -> &(Register2,Vec<RouteExpr>) {
        &self.route[reg]
    }
}