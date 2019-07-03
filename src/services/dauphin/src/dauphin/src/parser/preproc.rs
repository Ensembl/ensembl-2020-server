use super::node::{ Statement, ParseError };
use super::inline::{ check_inline_symbol, InlineMode, InlineStore, Inline };
use crate::lexer::Lexer;

fn run_import(path: &str, lexer: &mut Lexer) -> Result<(),Vec<ParseError>> {
    lexer.import(path).map_err(|s| vec![ParseError::new(&format!("import failed: {}",s),lexer)])
}

fn run_inline(symbol: &str, name: &str, mode: &InlineMode, prio: &f64, lexer: &mut Lexer, inlines: &mut InlineStore) -> Result<(),Vec<ParseError>> {
    check_inline_symbol(symbol)?; // TODO
    inlines.add(Inline {
        symbol: symbol.to_string(),
        name: name.to_string(),
        statement: false, // TODO
        precedence: *prio,
        mode: *mode
    });
    lexer.add_inline(symbol);
    Ok(())
}

pub fn preprocess(stmt: &Statement, lexer: &mut Lexer, inlines: &mut InlineStore) -> Result<bool,Vec<ParseError>> {
    match stmt {
        Statement::Import(path) => run_import(path,lexer).map(|_| true),
        Statement::Inline(symbol,name,mode,prio) => run_inline(&symbol,&name,mode,prio,lexer,inlines).map(|_| true),
        _ => { return Ok(false); }
    }
}