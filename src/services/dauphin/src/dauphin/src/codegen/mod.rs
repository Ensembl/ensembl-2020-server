mod definition;
mod definitionstore;
mod dename;
mod destructure;
mod generate;
mod instruction;
mod register;
mod simplify;
mod structenum;

pub use self::definition::{ Inline, InlineMode, ExprMacro, StmtMacro, ProcDecl, FuncDecl };
pub use self::definitionstore::DefStore;
pub use self::generate::Generator;
pub use self::register::{ Register, RegisterAllocator };
pub use self::instruction::Instruction;
pub use self::structenum::{ StructDef, EnumDef };
pub use self::simplify::simplify; // TODO remove
pub use self::dename::dename; // TODO remove