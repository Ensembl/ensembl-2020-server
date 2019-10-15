mod call;
mod codegen;
mod intstruction;
mod linearize;
mod optimise;
mod simplify;

pub use self::call::call;
pub use self::codegen::generate_code;
pub use self::intstruction::Instruction;
pub use self::linearize::linearize;
pub use self::optimise::remove_unused_registers;
pub use self::simplify::simplify;
