use std::collections::HashMap;
use super::node::ParseError;

#[derive(Debug,PartialEq,Clone,Copy)]
pub enum InlineMode {
    LeftAssoc,
    RightAssoc,
    Prefix
}

pub struct Inline {
    pub symbol: String,
    pub name: String,
    pub statement: bool,
    pub precedence: f64,
    pub mode: InlineMode
}

pub fn check_inline_symbol(sym: &str) -> Result<(),Vec<ParseError>> {
    Ok(())
}

pub struct InlineStore {
    inlines: HashMap<(String,bool),Inline>
}

impl InlineStore {
    pub fn new() -> InlineStore {
        InlineStore {
            inlines: HashMap::new()
        }
    }

    pub fn add(&mut self, token: Inline) {
        self.inlines.insert((token.symbol.clone(),token.statement),token);
    }
}

