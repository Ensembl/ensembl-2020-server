/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

#[macro_use]
pub mod polymorphic;


mod command;
mod commandsetid;
mod complexpath;
mod misc;
mod register;
mod templates;
mod types;
mod cborutil;
mod vectorsource;
mod sharedvec;
mod writevec;
mod vectorregisters;
mod fulltype;
mod registersignature;
mod vectorcopy;
mod xstructure;

pub use register::Register;
pub use templates::{ ErrorDeserializer, NoopDeserializer, ErrorInterpCommand, NoopInterpCommand };
pub use types::{ MemberMode, BaseType, MemberDataFlow };
pub use command::{ CommandDeserializer, InterpCommand };
pub use commandsetid::CommandSetId;
pub use cborutil::{
    cbor_int, cbor_array, cbor_bool, cbor_string, cbor_map, cbor_entry, cbor_type, CborType, cbor_map_iter, cbor_make_map, cbor_float,
    cbor_serialize
};
pub use misc::Identifier;
pub use sharedvec::SharedVec;
pub use writevec::WriteVec;
pub use vectorsource::{ VectorSource, RegisterVectorSource };
pub use vectorregisters::VectorRegisters;
pub use complexpath::ComplexPath;
pub use fulltype::FullType;
pub use registersignature::RegisterSignature;
pub use xstructure::{ to_xstructure, XStructure };
pub use vectorcopy::{ vector_update_poly, append_data };
pub use polymorphic::arbitrate_type;