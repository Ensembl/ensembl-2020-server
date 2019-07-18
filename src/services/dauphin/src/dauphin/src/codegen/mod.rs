mod definition;
mod definitionstore;
mod generate;
mod instruction;
mod register;
mod typeinf;
mod typepass;

pub use self::definition::{ Inline, InlineMode, ExprMacro, StmtMacro, ProcDecl, FuncDecl, StructDef, EnumDef };
pub use self::definitionstore::DefStore;
pub use self::generate::Generator;