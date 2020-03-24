mod call;
mod codegen;
mod dealias;
mod gencontext;
mod instruction;
mod linearize;
mod prune;
mod simplify;

pub use self::call::call;
pub use self::dealias::remove_aliases;
pub use self::gencontext::GenContext;
pub use self::codegen::generate_code;
pub use self::instruction::{ Instruction, InstructionType };
pub use self::linearize::linearize;
pub use self::prune::prune;
pub use self::simplify::simplify;
