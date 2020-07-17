#[macro_use]
pub mod polymorphic;

mod complexpath;
mod fulltype;
mod registersignature;
mod sharedvec;
mod types;
mod vectorcopy;
mod vectorregisters;
mod vectorsource;
mod writevec;
mod xstructure;

pub use self::complexpath::ComplexPath;
pub use self::fulltype::FullType;
pub use self::polymorphic::{ arbitrate_type };
pub use self::registersignature::RegisterSignature;
pub use self::sharedvec::SharedVec;
pub use self::types::{ BaseType, MemberMode, MemberDataFlow };
pub use self::vectorcopy::{ vector_update_poly, append_data };
pub use self::vectorregisters::VectorRegisters;
pub use self::vectorsource::{ RegisterVectorSource, VectorSource };
pub use self::writevec::WriteVec;
pub use self::xstructure::{ XStructure, to_xstructure };