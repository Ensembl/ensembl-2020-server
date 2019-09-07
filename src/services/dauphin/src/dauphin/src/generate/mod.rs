mod codegen;
mod intstruction;
mod simplify;

pub use self::codegen::generate_code;
pub use self::intstruction::Instruction;
pub use self::simplify::Extender; // XXX