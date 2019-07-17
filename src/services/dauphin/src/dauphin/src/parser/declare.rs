use super::node::{ ParserStatement, ParseError, Type, Sig };
use super::lexutil::not_reserved;
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
    lexer.add_inline(symbol,mode == &InlineMode::Prefix).map_err(|s| {
        ParseError::new(&s,lexer)
    })?;
    defstore.add_inline(Inline::new(symbol,name,stmt_like,prio,mode))?;
    Ok(())
}

fn run_expr(name: &str, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    defstore.add_expr(ExprMacro::new(name),lexer)?;
    Ok(())
}

fn run_stmt(name: &str, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    defstore.add_stmt(StmtMacro::new(name),lexer)?;
    Ok(())
}

fn run_proc(name: &str, sigs: &Vec<Sig>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    defstore.add_proc(ProcDecl::new(name,sigs),lexer)?;
    Ok(())
}

fn run_func(name: &str, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    defstore.add_func(FuncDecl::new(name),lexer)?;
    Ok(())
}

fn run_struct(name: &str, types: &Vec<Type>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    let def = StructDef::new(name,types,names,defstore).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_struct(def,lexer)?;
    Ok(())
}

fn run_enum(name: &str, types: &Vec<Type>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    let def = EnumDef::new(name,types,names,defstore).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_enum(def,lexer)?;
    Ok(())
}

pub fn declare(stmt: &ParserStatement, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<bool,ParseError> {
    match stmt {
        ParserStatement::Import(path) =>
            run_import(path,lexer).map(|_| true),
        ParserStatement::Inline(symbol,name,mode,prio) => 
            run_inline(&symbol,&name,mode,*prio,lexer,defstore).map(|_| true),
        ParserStatement::ExprMacro(name) =>
            run_expr(&name,defstore,lexer).map(|_| true),
        ParserStatement::StmtMacro(name) =>
            run_stmt(&name,defstore,lexer).map(|_| true),
        ParserStatement::ProcDecl(name,sigs) =>
            run_proc(&name,sigs,defstore,lexer).map(|_| true),
        ParserStatement::FuncDecl(name) =>
            run_func(&name,defstore,lexer).map(|_| true),
        ParserStatement::StructDef(name,types,names) =>
            run_struct(&name,&types,&names,defstore,lexer).map(|_| true),
        ParserStatement::EnumDef(name,types,names) =>
            run_enum(&name,types,names,defstore,lexer).map(|_| true),
        _ => { return Ok(false); }
    }
}
