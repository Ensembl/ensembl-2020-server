mod cborutil;
mod definition;
mod definitionstore;
mod register;
mod structenum;
mod signature {
    pub mod signature;
    pub mod complexsig;
    pub mod vectorsig;
}

pub use self::definition::{ Inline, InlineMode, ExprMacro, StmtMacro, ProcDecl, FuncDecl };
pub use self::definitionstore::DefStore;
pub use self::signature::signature::RegisterSignature;
pub use self::signature::complexsig::ComplexRegisters;
pub use self::signature::vectorsig::VectorRegisters;
pub use self::register::{ Register, RegisterAllocator };
pub use self::structenum::{ StructDef, EnumDef };
pub use self::cborutil::{ cbor_int, cbor_array, cbor_string };