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

mod assignregs;
mod call;
mod codegen;
mod dealias;
mod gencontext;
mod generate;
mod linearize;
mod pauses;
mod peephole;
mod prune;
mod compilerun;
mod simplify;
mod retreat;
mod reusedead;
mod reuseregs;
mod useearliest;

pub use self::gencontext::GenContext;
pub use self::generate::generate;

// For testing in another crate
pub use self::codegen::generate_code;
pub use self::simplify::simplify;
pub use self::call::call;
