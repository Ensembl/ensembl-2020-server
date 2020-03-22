mod call;
mod codegen;
mod instruction;
mod linearize;
mod optimise;
mod simplify;

pub use self::call::call;
pub use self::codegen::{ generate_code, GenContext }; // XXX don't export GenContext
pub use self::instruction::{ Instruction, InstructionType };
pub use self::linearize::linearize;
pub use self::optimise::remove_unused_registers;
pub use self::simplify::simplify;
