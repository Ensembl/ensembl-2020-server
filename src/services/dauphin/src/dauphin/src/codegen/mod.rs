mod definition;
mod definitionstore;

pub use self::definition::{ Inline, InlineMode, ExprMacro, StmtMacro, ProcDecl, FuncDecl, StructDef, EnumDef };
pub use self::definitionstore::DefStore;