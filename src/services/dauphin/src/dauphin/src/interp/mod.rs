mod command;
mod context;
mod harness;
mod stream;
mod commands {
    pub mod core;
    pub mod assign;
    pub mod library;
}
mod values {
    pub mod registers;
    pub mod supercow;
    pub mod value;

}
pub use self::harness::mini_interp;
pub use self::values::value::{ to_index, InterpValue, InterpNatural };
pub use self::values::registers::RegisterFile;
pub use self::stream::StreamContents;