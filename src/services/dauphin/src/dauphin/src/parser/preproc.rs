use super::node::{ ParserStatement, ParseError };
use crate::codegen::{
    InlineMode, Inline, DefStore, ExprMacro, StmtMacro, FuncDecl, ProcDecl,
    StructDef, EnumDef
};
use crate::lexer::Lexer;

fn run_import(path: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.import(path).map_err(|s| ParseError::new(&format!("import failed: {}",s),lexer))
}

fn run_inline(symbol: &str, name: &str, mode: &InlineMode, prio: f64, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<(),ParseError> {
    let stmt_like = defstore.stmt_like(name,lexer)?;
    defstore.add_inline(Inline::new(symbol,name,stmt_like,prio,mode));
    lexer.add_inline(symbol);
    Ok(())
}

fn run_expr(name: &str, defstore: &mut DefStore, lexer: &Lexer) -> Result<(),ParseError> {
    defstore.add_expr(ExprMacro::new(name),lexer)?;
    Ok(())
}

fn run_stmt(name: &str, defstore: &mut DefStore, lexer: &Lexer) -> Result<(),ParseError> {
    defstore.add_stmt(StmtMacro::new(name),lexer)?;
    Ok(())
}

fn run_proc(name: &str, defstore: &mut DefStore, lexer: &Lexer) -> Result<(),ParseError> {
    defstore.add_proc(ProcDecl::new(name),lexer)?;
    Ok(())
}

fn run_func(name: &str, defstore: &mut DefStore, lexer: &Lexer) -> Result<(),ParseError> {
    defstore.add_func(FuncDecl::new(name),lexer)?;
    Ok(())
}

fn run_struct(name: &str, defstore: &mut DefStore, lexer: &Lexer) -> Result<(),ParseError> {
    defstore.add_struct(StructDef::new(name),lexer)?;
    Ok(())
}

fn run_enum(name: &str, defstore: &mut DefStore, lexer: &Lexer) -> Result<(),ParseError> {
    defstore.add_enum(EnumDef::new(name),lexer)?;
    Ok(())
}

pub fn preprocess(stmt: &ParserStatement, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<bool,ParseError> {
    match stmt {
        ParserStatement::Import(path) =>
            run_import(path,lexer).map(|_| true),
        ParserStatement::Inline(symbol,name,mode,prio) => 
            run_inline(&symbol,&name,mode,*prio,lexer,defstore).map(|_| true),
        ParserStatement::ExprMacro(name) =>
            run_expr(&name,defstore,lexer).map(|_| true),
        ParserStatement::StmtMacro(name) =>
            run_stmt(&name,defstore,lexer).map(|_| true),
        ParserStatement::ProcDecl(name) =>
            run_proc(&name,defstore,lexer).map(|_| true),
        ParserStatement::FuncDecl(name) =>
            run_func(&name,defstore,lexer).map(|_| true),
        ParserStatement::StructDef(name) =>
            run_struct(&name,defstore,lexer).map(|_| true),
        ParserStatement::EnumDef(name) =>
            run_enum(&name,defstore,lexer).map(|_| true),
        _ => { return Ok(false); }
    }
}