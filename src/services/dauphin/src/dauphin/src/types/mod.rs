mod typeinf;
mod typepass;
mod typestep;
mod uniquifier;
mod types;

// TODO remove
pub use self::typeinf::Referrer;
pub use self::typepass::TypePass;
pub use types::{ Type, BaseType, Sig, TypeSig, TypeSigExpr };