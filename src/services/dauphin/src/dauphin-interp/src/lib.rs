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
extern crate dauphin_interp_common;

pub mod core {
    pub mod core;
    mod consts;
    mod commontype;
    pub use self::core::make_core_interp;
}

pub mod interp {
    mod deserializer;
    mod interpretsuite;
    mod interplink;
    mod interpret;
    pub use self::deserializer::Deserializer;
    pub use self::interpretsuite::CommandInterpretSuite;
    pub use self::interplink::InterpreterLink;
    pub use self::interpret::{ StandardInterpretInstance, DebugInterpretInstance, InterpretInstance };
}

pub use self::core::make_core_interp;

#[cfg(test)]
mod test;
