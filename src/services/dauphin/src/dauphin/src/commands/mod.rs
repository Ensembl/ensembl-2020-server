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
mod common {
    #[macro_use]
    pub(super) mod polymorphic;
    pub(super) mod templates;
    pub(super) mod sharedvec;
    pub(super) mod vectorcopy;
    pub(super) mod vectorsource;
    pub(super) mod writevec;
    pub(super) mod expandedsig;
}
mod core {
    #[macro_use]
    pub(super) mod commontype;
    pub(super) mod consts;
    pub(super) mod core;
}

mod std {
    pub(super) mod assign;
    pub(super) mod extend;
    pub(super) mod print;
    pub(super) mod vector;
    pub(super) mod library;
    mod numops;
    mod eq;
}

mod buildtime {
    pub(super) mod buildtime;
    pub(super) mod ini;
    pub(super) mod dump;
    pub(super) mod defines;
    pub(super) mod versions;
    pub(super) mod hints;
}

// XXX unexport
pub use self::core::consts::{
    ConstCommandType, NumberConstCommandType, BooleanConstCommandType, StringConstCommandType
};
pub use self::core::core::{ make_core, make_core_interp };
pub use self::std::library::{ make_std, make_std_interp, std_stream };
pub use self::buildtime::buildtime::{ make_buildtime };
pub use self::common::expandedsig::{ XStructure, to_xstructure };
pub use self::common::templates::{ ErrorInterpCommand, NoopDeserializer, ErrorDeserializer };