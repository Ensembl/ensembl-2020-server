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

mod cborutil;
mod definition;
mod definitionstore;
mod dfloat;
mod identifierstore;
mod register;
mod structenum;
mod signature {
    pub mod complexpath;
    pub mod signature;
    pub mod complexsig;
    pub mod vectorsig;
}

pub use self::definition::{ Inline, InlineMode, ExprMacro, StmtMacro, ProcDecl, FuncDecl };
pub use self::definitionstore::DefStore;
pub use self::identifierstore::{ IdentifierPattern, Identifier, IdentifierStore, IdentifierUse };
pub use self::signature::complexpath::ComplexPath;
pub use self::signature::signature::RegisterSignature;
pub use self::signature::complexsig::ComplexRegisters;
pub use self::signature::vectorsig::VectorRegisters;
pub use self::register::{ Register, RegisterAllocator };
pub use self::structenum::{ StructDef, EnumDef };
pub use self::dfloat::DFloat;
pub use self::cborutil::{ cbor_int, cbor_array, cbor_bool, cbor_string, cbor_map, cbor_entry, cbor_type, CborType, cbor_map_iter, cbor_make_map, cbor_float };