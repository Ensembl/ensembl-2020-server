mod call;
mod codegen;
mod gencontext;
mod instruction;
mod linearize;
mod optimise;
mod simplify;

pub use self::call::call;
pub use self::gencontext::GenContext;
pub use self::codegen::generate_code; // XXX don't export GenContext
pub use self::instruction::{ Instruction, InstructionType };
pub use self::linearize::linearize;
pub use self::optimise::remove_unused_registers;
pub use self::simplify::simplify;
