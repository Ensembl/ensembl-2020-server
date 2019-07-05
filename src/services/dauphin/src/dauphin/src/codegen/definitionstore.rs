use std::collections::HashMap;
use std::collections::hash_map::Entry;
use super::definition::{
    ExprMacro, StmtMacro, FuncDecl, ProcDecl, StructDef, EnumDef, Inline,
    check_inline_symbol, InlineMode
};
use crate::lexer::Lexer;
use crate::parser::ParseError;

pub struct DefStore {
    namespace: HashMap<String,(String,u32,u32)>,
    exprs: HashMap<String,ExprMacro>,
    stmts: HashMap<String,StmtMacro>,
    funcs: HashMap<String,FuncDecl>,
    procs: HashMap<String,ProcDecl>,
    structs: HashMap<String,StructDef>,
    enums: HashMap<String,EnumDef>,
    inlines_binary: HashMap<String,Inline>,
    inlines_unary: HashMap<String,Inline>
}

impl DefStore {
    pub fn new() -> DefStore {
        DefStore {
            namespace: HashMap::new(),
            exprs: HashMap::new(),
            stmts: HashMap::new(),
            funcs: HashMap::new(),
            procs: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            inlines_binary: HashMap::new(),
            inlines_unary: HashMap::new(),
        }
    }

    fn detect_clash(&mut self, cmp: &str, lexer: &Lexer) -> Result<(),ParseError> {
        match self.namespace.entry(cmp.to_string()) {
            Entry::Occupied(e) => {
                let (file,line,col) = e.get();
                Err(ParseError::new(
                    &format!("'{}' already defined at {} {}:{}",
                        cmp,file,line,col),lexer))
            },
            Entry::Vacant(e) => {
                let (file,line,col) = lexer.position();
                e.insert((file.to_string(),line,col));
                Ok(())
            }
        }
    }

    pub fn add_expr(&mut self, expr: ExprMacro, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(expr.name(),lexer)?;
        self.exprs.insert(expr.name().to_string(),expr);
        Ok(())
    }

    pub fn add_stmt(&mut self, stmt: StmtMacro, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(stmt.name(),lexer)?;
        self.stmts.insert(stmt.name().to_string(),stmt);
        Ok(())
    }

    pub fn add_func(&mut self, func: FuncDecl, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(func.name(),lexer)?;
        self.funcs.insert(func.name().to_string(),func);
        Ok(())
    }

    pub fn add_proc(&mut self, proc_: ProcDecl, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(proc_.name(),lexer)?;
        self.procs.insert(proc_.name().to_string(),proc_);
        Ok(())
    }

    pub fn add_struct(&mut self, struct_: StructDef, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(struct_.name(),lexer)?;
        self.structs.insert(struct_.name().to_string(),struct_);
        Ok(())
    }

    pub fn add_enum(&mut self, enum_: EnumDef, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(enum_.name(),lexer)?;
        self.enums.insert(enum_.name().to_string(),enum_);
        Ok(())
    }

    pub fn add_inline(&mut self, inline: Inline) -> Result<(),ParseError> {
        check_inline_symbol(&inline.symbol())?; // TODO
        if inline.mode() == &InlineMode::Prefix {
            self.inlines_unary.insert(inline.symbol().to_string(),inline);
        } else {
            self.inlines_binary.insert(inline.symbol().to_string(),inline);
        }
        Ok(())
    }

    pub fn get_inline_binary(&self, symbol: &str, lexer: &Lexer) -> Result<&Inline,ParseError> {
        self.inlines_binary.get(symbol).ok_or(
            ParseError::new(&format!("No such binary operator: {}",symbol),lexer)
        )
    }

    pub fn get_inline_unary(&self, symbol: &str, lexer: &Lexer) -> Result<&Inline,ParseError> {
        self.inlines_unary.get(symbol).ok_or(
            ParseError::new(&format!("No such unary operator: {}",symbol),lexer)
        )
    }

    pub fn stmt_like(&self, cmp: &str, lexer: &Lexer) -> Result<bool,ParseError> {
        if self.stmts.contains_key(cmp) || self.procs.contains_key(cmp) {
            Ok(true)
        } else if self.exprs.contains_key(cmp) || self.funcs.contains_key(cmp) {
            Ok(false)
        } else {
            Err(ParseError::new(&format!("No such symbol: '{}'",cmp),lexer))
        }
    }
}
