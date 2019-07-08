mod definition;
mod definitionstore;
mod generate;
mod instruction;
mod register;

pub use self::definition::{ Inline, InlineMode, ExprMacro, StmtMacro, ProcDecl, FuncDecl, StructDef, EnumDef };
pub use self::definitionstore::DefStore;